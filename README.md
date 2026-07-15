# CodeStream — Distributed Collaborative Code Editor

Platforma za kolaborativno programiranje u realnom vremenu. Više korisnika može istovremeno da uređuje isti kod bez konflikata i odmah da ga izvršava direktno u browser-u.

---

## Arhitektura sistema

Sistem je implementiran kao **mikroservisna aplikacija u Rust-u**, sastavljena od 4 nezavisna servisa koji komuniciraju asinhrono. Sav saobraćaj iz browser-a prolazi kroz **nginx API gateway** kao jedinu ulaznu tačku.

```
Browser (localhost:5500)
        │
        ▼
┌─────────────────────────┐
│   nginx — API Gateway   │
│  /api/auth/*  → :3000   │
│  /api/crdt/*  → :3002   │
│  /api/exec/*  → :3003   │
│  /ws/*        → :3001   │
└─────────────────────────┘
        │  interna Docker mreža
        ├──► auth-user-service:3000      → PostgreSQL
        ├──► session-gateway-service:3001 → RabbitMQ
        ├──► crdt-sync-service:3002      → Redis + RabbitMQ
        └──► code-execution-service:3003 → Docker sandbox
```

### Servisi

#### A. Auth & User Service (port 3000)

Jedini servis koji direktno komunicira sa relacionom bazom. Zadužen za identitet korisnika i upravljanje projektima.

- Registracija i login korisnika sa **Argon2** hashingom lozinke
- Izdavanje i validacija **JWT tokena** (HS256, TTL 24h)
- CRUD operacije nad projektima u **PostgreSQL**-u
- Upravljanje pravima pristupa: `owner` / `write` / `read`
- SQL šema se primenjuje automatski pri pokretanju kroz `sqlx::migrate!()`

| Metoda | Ruta | Opis |
|---|---|---|
| POST | `/register` | Kreiranje naloga |
| POST | `/login` | Prijava, vraća JWT |
| GET | `/me` | Podaci o ulogovanom korisniku |
| GET | `/projects` | Lista projekata korisnika |
| POST | `/projects` | Novi projekat |
| GET | `/projects/:id` | Jedan projekat (koristi i gateway) |
| POST | `/projects/:id/access` | Dodela pristupa |

#### B. Session Gateway Service (port 3001)

Jedina real-time veza između browser-a i sistema. Upravlja WebSocket konekcijama i posreduje između browsera i CRDT servisa kroz RabbitMQ.

- Prima **WebSocket** konekcije, validira JWT token
- Sinhrono pita auth servis za pristup projektu (HTTP poziv ka `:3000`)
- Upravlja "sobama" po projektu: `Arc<RwLock<HashMap<Uuid, broadcast::Sender<String>>>>`
- Publish-uje CRDT operacije na `crdt.operations` red (fire-and-forget)
- U zasebnom Tokio tasku konzumira `crdt.results` red i broadcast-uje rezultate svim korisnicima u sobi
- Prosleđuje cursor pozicije i promene jezika između korisnika

#### C. CRDT Sync Service (port 3002)

Servis koji rešava konflikte pri simultanom uređivanju teksta.

- Implementira **RGA (Replicated Growable Array)** algoritam
- Svaki karakter ima jedinstven `CharId { clock: u64, user_id: Uuid }` — Lamport-ov logički sat
- Konflikti se rešavaju deterministički: veći `clock` ima prioritet, pri jednakosti veći UUID
- Obrisani karakteri ostaju kao **tombstone** (`deleted: true`) jer kasne operacije mogu referencirati obrisane karaktere
- Stanje dokumenta se serijalizuje u JSON i čuva u **Redis**-u pod ključem `doc:{project_id}`
- Konzumira `crdt.operations` red iz RabbitMQ-a, obrađuje operaciju, publish-uje na `crdt.results`
- HTTP endpoint `GET /documents/:id/state` za inicijalno učitavanje dokumenta pri otvaranju editora

#### D. Code Execution Service (port 3003)

Bezbedno, izolovano izvršavanje koda koji korisnici napišu.

- Prima `POST /execute` sa `{ language, code, timeout_seconds }`
- Kreira Docker kontejner sa restrikcijama:
  - RAM: **128 MB**
  - CPU: **50% jednog core-a**
  - Mreža: **`none`** — bez internet pristupa
- Podržani jezici: **Python** (`python:3.12-slim`) i **JavaScript** (`node:20-slim`)
- Vraća `stdout`, `stderr`, `exit_code` i `execution_time_ms`
- Limit: 64 KB koda, default timeout 10s (max 30s)
- Komunicira sa Docker daemon-om kroz Unix socket koristeći `bollard` biblioteku

### Shared Crate

Zajednička biblioteka (`services/shared`) koju koriste sva 4 servisa:

- `AuthUser` — Axum extractor koji automatski validira JWT iz `Authorization` headera
- `Claims` — JWT payload struct (`sub`, `email`, `exp`)
- `CrdtOperationMessage` / `CrdtResultMessage` — deljeni tipovi poruka za RabbitMQ

---

## Komunikacija između servisa

### Sinhrona (HTTP)
Gateway → Auth servis: proverava pristup projektu pri svakoj WebSocket konekciji.

### Asinhrona (RabbitMQ)
Dva reda:
- `crdt.operations` — Gateway publish-uje operacije, CRDT servis konzumira
- `crdt.results` — CRDT servis publish-uje rezultate, Gateway konzumira i broadcast-uje

---

## Tehnološki stack

| Komponenta | Tehnologija |
|---|---|
| Programski jezik (backend) | Rust (Axum, Tokio, sqlx, lapin, bollard) |
| Frontend | React 18, Vite |
| API Gateway | nginx (reverse proxy + SPA serving) |
| Message broker | RabbitMQ 3 |
| Relaciona baza | PostgreSQL 16 |
| In-memory baza | Redis 7 |
| Kontejnerizacija | Docker, Docker Compose |
| Autentifikacija | JWT (HS256), Argon2 hashing |
| CRDT algoritam | RGA (Replicated Growable Array) |

---

## Pokretanje aplikacije

### Preduslovi

- [Docker](https://www.docker.com/) i Docker Compose v2

### Pokretanje

```bash
git clone <repo-url>
cd CodeStream

docker compose up -d
```

Docker Compose podiže ceo sistem: PostgreSQL, Redis, RabbitMQ, sva 4 servisa i nginx frontend. Sačekati ~30 sekundi za inicijalizaciju RabbitMQ-a.

```bash
# Provera statusa svih kontejnera
docker compose ps

# Logovi određenog servisa
docker compose logs -f session-gateway-service
```

### Ulazne tačke

| | URL |
|---|---|
| **Aplikacija (API Gateway)** | **http://localhost:5500** |
| RabbitMQ Management UI | http://localhost:15672 (guest / guest) |

> Sav saobraćaj ide kroz port **5500**. Portovi 3000–3003 su interno dostupni ali nisu potrebni za korišćenje aplikacije.

### Zaustavljanje

```bash
# Zaustavi (podaci ostaju u Docker volumima)
docker compose down

# Zaustavi i obriši sve podatke
docker compose down -v
```

### Lokalni razvoj (bez Docker-a)

Zahteva lokalno instalirane: PostgreSQL, Redis, RabbitMQ.

Kreirati `.env` fajlove:

**`services/auth-user-service/.env`**
```
DATABASE_URL=postgres://postgres:postgres@localhost:5432/codestream
JWT_SECRET=codestream_dev_secret
```

**`services/session-gateway-service/.env`**
```
JWT_SECRET=codestream_dev_secret
AUTH_SERVICE_URL=http://localhost:3000
AMQP_URL=amqp://guest:guest@localhost:5672/%2f
```

**`services/crdt-sync-service/.env`**
```
REDIS_URL=redis://localhost:6379
AMQP_URL=amqp://guest:guest@localhost:5672/%2f
JWT_SECRET=codestream_dev_secret
```

**`services/code-execution-service/.env`**
```
JWT_SECRET=codestream_dev_secret
```

```bash
# Servisi (iz foldera services/, svaki u posebnom terminalu)
cargo run --bin auth-user-service
cargo run --bin session-gateway-service
cargo run --bin crdt-sync-service
cargo run --bin code-execution-service

# Frontend (iz foldera frontend/)
npm install
npm run dev    # http://localhost:5500 — Vite proxy preuzima ulogu API gatewaya
```

---

## Plan za diplomski rad

Nakon uspešne odbrane projektnog zadatka, planira se proširenje sistema u diplomski rad:

**"Implementacija distribuiranog sistema za kolaboraciju u realnom vremenu zasnovanog na CRDT strukturama"**

### 1. Optimizacija RGA algoritma i upravljanje tombstone-ovima

Trenutna implementacija čuva sve obrisane karaktere zauvek (tombstone pristup). Kod dugotrajnih sesija sa intenzivnim uređivanjem, RGA struktura neprestano raste u memoriji.

Planirana proširenja:
- Implementacija **tombstone garbage collection** — bezbedno uklanjanje tombstone-ova za koje je garantovano da ih nijedan aktivan čvor više ne referencira
- Istraživanje alternativnih CRDT struktura: **YATA** (Yet Another Transformation Approach, koji koristi Yjs) i **RGA-Split** koji bolje performira pri paralelnim insert operacijama na istoj poziciji
- Benchmarking i analiza performansi pod opterećenjem: merenje latencije konvergencije pri 10, 50 i 100 simultanih korisnika

### 2. Offline-first podrška i delta sinhronizacija

Trenutni sistem zahteva stalnu konekciju. Prekid znači gubitak promena.

Planirana proširenja:
- Lokalno čuvanje nepotvrđenih operacija u browser-u (`IndexedDB`) kada WebSocket konekcija padne
- Automatsko slanje nakupljenih operacija pri ponovnom uspostavljanju konekcije
- **Delta sinhronizacija** — pri reconnect-u razmena samo razlike u stanju (vector clock poređenje), a ne celog dokumenta
- Validacija da komutativnost CRDT operacija garantuje ispravnu konvergenciju bez obzira na redosled prispeća

### 3. Event sourcing i istorija verzija

Trenutno Redis čuva samo trenutno stanje dokumenta, nema istorije promena.

Planirana proširenja:
- Svaka potvrđena CRDT operacija se trajno čuva u PostgreSQL-u kao nepromenljivi event (`operation_log` tabela)
- **"Time travel"** — korisnik može da vrati dokument na bilo koje prethodno stanje reprodukovanjem event loga
- Vizualizacija istorije promena po korisniku (ko je šta i kada izmenio)
- Mogućnost grananja verzija, eksperimentalna izmena u zasebnoj kopiji dokumenta koja se može merge-ovati nazad
