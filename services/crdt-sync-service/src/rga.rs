use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique, totally-ordered identifier for every character in the document.
/// `clock` is the document's logical clock at the moment of insertion.
/// When two concurrent inserts produce the same clock value, `user_id` breaks
/// the tie deterministically — so every replica orders them identically.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CharId {
    pub clock: u64,
    pub user_id: Uuid,
}

impl PartialOrd for CharId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CharId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher clock wins; equal clock → larger user_id wins (arbitrary but stable).
        self.clock
            .cmp(&other.clock)
            .then_with(|| self.user_id.cmp(&other.user_id))
    }
}

/// One node in the RGA sequence.
/// Deleted characters stay in the list as "tombstones" so that concurrent
/// delete/insert operations on the same position can still be resolved.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RgaChar {
    pub id: CharId,
    pub content: char,
    pub deleted: bool,
    /// The id of the character this node was inserted after.
    /// `None` means it was inserted at the very beginning of the document.
    pub prev_id: Option<CharId>,
}

/// Full document state persisted in Redis.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RgaDocument {
    /// All characters in RGA order, including tombstones.
    pub chars: Vec<RgaChar>,
    /// Monotonically increasing logical clock for this document.
    pub clock: u64,
}

// ---------------------------------------------------------------------------
// Operation types
// ---------------------------------------------------------------------------

/// What a client sends: either insert a character after a known position,
/// or delete a character by its server-assigned id.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientOperation {
    Insert {
        /// The id of the predecessor character; `None` = insert at document head.
        after_char_id: Option<CharId>,
        content: char,
    },
    Delete {
        char_id: CharId,
    },
}

/// What the server sends back after applying the operation.
/// For inserts the server fills in the `char_id` (from the logical clock)
/// so every client can refer to this character by its stable identity later.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResolvedOperation {
    Insert {
        char_id: CharId,
        after_char_id: Option<CharId>,
        content: char,
    },
    Delete {
        char_id: CharId,
    },
}

// ---------------------------------------------------------------------------
// RGA algorithm
// ---------------------------------------------------------------------------

impl RgaDocument {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a client operation to the document.
    /// Returns the resolved operation (with server-assigned `char_id` for inserts).
    pub fn apply(
        &mut self,
        op: ClientOperation,
        user_id: Uuid,
    ) -> Result<ResolvedOperation, String> {
        match op {
            ClientOperation::Insert {
                after_char_id,
                content,
            } => {
                self.clock += 1;
                let char_id = CharId {
                    clock: self.clock,
                    user_id,
                };
                self.insert_char(char_id.clone(), content, after_char_id.clone())?;
                Ok(ResolvedOperation::Insert {
                    char_id,
                    after_char_id,
                    content,
                })
            }
            ClientOperation::Delete { char_id } => {
                self.delete_char(&char_id)?;
                Ok(ResolvedOperation::Delete { char_id })
            }
        }
    }

    /// Insert a new character into the RGA sequence.
    ///
    /// RGA concurrent-insert rule: characters that share the same predecessor
    /// are ordered by their `CharId` descending (higher id = earlier in text).
    /// This guarantees convergence regardless of the order operations arrive.
    fn insert_char(
        &mut self,
        id: CharId,
        content: char,
        prev_id: Option<CharId>,
    ) -> Result<(), String> {
        // Find the index right after the predecessor.
        let start_idx = match &prev_id {
            None => 0,
            Some(pid) => self
                .chars
                .iter()
                .position(|c| &c.id == pid)
                .ok_or("Predecessor character not found")?
                + 1,
        };

        // Skip past any concurrent characters at the same position that have
        // higher priority (larger CharId), so they remain to the left of us.
        let insert_at = self.chars[start_idx..]
            .iter()
            .enumerate()
            .take_while(|(_, c)| c.prev_id == prev_id && c.id > id)
            .map(|(i, _)| start_idx + i + 1)
            .last()
            .unwrap_or(start_idx);

        self.chars.insert(
            insert_at,
            RgaChar {
                id,
                content,
                deleted: false,
                prev_id,
            },
        );
        Ok(())
    }

    /// Mark a character as deleted (tombstone).
    /// The node is kept so that operations referencing it as a predecessor
    /// can still be resolved correctly.
    fn delete_char(&mut self, char_id: &CharId) -> Result<(), String> {
        self.chars
            .iter_mut()
            .find(|c| &c.id == char_id)
            .ok_or_else(|| "Character not found".to_string())
            .map(|c| c.deleted = true)
    }

    /// Returns only the visible (non-deleted) characters as a plain string.
    pub fn text(&self) -> String {
        self.chars
            .iter()
            .filter(|c| !c.deleted)
            .map(|c| c.content)
            .collect()
    }
}
