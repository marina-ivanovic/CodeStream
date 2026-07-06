use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        self.clock
            .cmp(&other.clock)
            .then_with(|| self.user_id.cmp(&other.user_id))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RgaChar {
    pub id: CharId,
    pub content: char,
    pub deleted: bool,
    pub prev_id: Option<CharId>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RgaDocument {
    pub chars: Vec<RgaChar>,
    pub clock: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientOperation {
    Insert {
        after_char_id: Option<CharId>,
        content: char,
    },
    Delete {
        char_id: CharId,
    },
}

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

impl RgaDocument {
    pub fn new() -> Self {
        Self::default()
    }

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

    fn insert_char(
        &mut self,
        id: CharId,
        content: char,
        prev_id: Option<CharId>,
    ) -> Result<(), String> {
        let start_idx = match &prev_id {
            None => 0,
            Some(pid) => self
                .chars
                .iter()
                .position(|c| &c.id == pid)
                .ok_or("Predecessor character not found")?
                + 1,
        };

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

    fn delete_char(&mut self, char_id: &CharId) -> Result<(), String> {
        self.chars
            .iter_mut()
            .find(|c| &c.id == char_id)
            .ok_or_else(|| "Character not found".to_string())
            .map(|c| c.deleted = true)
    }

    pub fn text(&self) -> String {
        self.chars
            .iter()
            .filter(|c| !c.deleted)
            .map(|c| c.content)
            .collect()
    }
}
