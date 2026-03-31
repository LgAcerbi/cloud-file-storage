# Cloud File Storage Platform Plan

## Goal

Design and build a cloud-based object storage and file sync platform inspired by products like OneDrive and Dropbox.

This project is in the planning phase only. No implementation code is defined here beyond architectural direction and development milestones.

## Product Vision

Build a secure, scalable platform that allows users and teams to:

- upload and download files from any device
- organize files and folders
- synchronize file changes across clients
- share files and folders with other users
- preserve file history and support recovery
- expose the platform through REST APIs and future desktop/mobile clients

## Technology Direction

### Required Languages

- `Rust` for operating system and filesystem interaction
- `Java` for REST APIs and service orchestration

### Why This Split

- `Rust` is well suited for efficient, safe interaction with filesystems, local agents, background sync logic, metadata scanning, chunking, hashing, and other performance-sensitive operations.
- `Java` is well supported for HTTP APIs on the JVM, with mature libraries for authentication, admin endpoints, metadata services, sharing features, and integration with any client (web, desktop, or mobile) that consumes REST.

## High-Level System Concept

The platform should be designed as a set of cooperating services rather than a single monolith from day one, even if early development starts in a modular monorepo.

### Core Building Blocks

1. `API Gateway / REST API` in Java
2. `Auth and Identity Service` in Java
3. `Metadata Service` in Java
4. `Object Storage Layer` backed by cloud blob/object storage
5. `Sync/Agent Service` or local sync client logic in Rust
6. `OS Interaction Layer` in Rust for filesystem watching and local file operations
7. `Background Jobs / Event Processing` for indexing, versioning, virus scanning, previews, and cleanup
8. `Database` for metadata, permissions, versions, jobs, and audit logs
9. `Cache / Queue / Event Bus` as needed for scale

## Scope Definition

### In Scope for Initial Product

- personal file storage
- folders and nested organization
- upload and download
- resumable large file uploads
- object metadata management
- file versioning
- soft delete and recovery
- file and folder sharing by link or direct permission
- sync agent foundation
- audit and activity logging
- admin observability basics

### Deferred Until Final Phase

- user accounts and authentication
- role and permission hardening
- advanced admin controls tied to identity governance

### Out of Scope for First Release

- collaborative document editing
- office suite replacement
- AI-based search or summarization
- public marketplace or third-party plugin ecosystem
- complex enterprise compliance certifications at launch

## Functional Requirements

### 1. Identity and Access (Final Phase)

- Users can register, sign in, sign out, and reset passwords.
- The system supports roles such as user, admin, and support operator.
- Authorization must support ownership, per-file access, and shared folder access.
- Sessions and tokens must support expiration and revocation.

### 2. File and Folder Management

- Users can create, rename, move, copy, and delete folders.
- Users can upload one or many files.
- Users can replace existing files and create new versions.
- Users can browse directory structures with pagination for large folders.
- Users can view metadata such as name, size, hash, media type, timestamps, owner, and version.

### 3. Upload and Download

- Support single-part upload for small files.
- Support multipart or chunked upload for large files.
- Support resumable upload after interruption.
- Validate checksum or hash integrity on upload completion.
- Downloads should support range requests for large files and media streaming.

### 4. Sync

- Detect local file changes through Rust-based OS watchers.
- Detect remote changes and reconcile them locally.
- Handle create, update, delete, rename, and move operations.
- Resolve conflicts with a clear strategy and user-visible conflict artifacts when needed.
- Maintain a local state database or cache for sync bookkeeping.

### 5. Sharing and Permissions

- Users can share files or folders directly with other users.
- Users can generate time-limited or revocable share links.
- Shared resources support view-only and edit permissions where appropriate.
- Access events should be auditable.

### 6. Versioning and Recovery

- Each update creates a new file version according to retention policy.
- Users can view version history and restore an earlier version.
- Deleted items move to a recycle bin before permanent deletion.
- Background lifecycle jobs enforce retention and cleanup.

### 7. Search and Discovery

- Search by file and folder name in the first release.
- Future support can include tag, content, and semantic search.

### 8. Administration and Operations

- Admins can view system health, storage usage, user activity, and failed jobs.
- Admins can suspend users, revoke shares, and inspect audit history.
- System must emit logs, metrics, and traces suitable for production monitoring.

Planning priority note:

- identity and full user management are intentionally implemented last to keep the primary focus on object/blob storage and OS-level sync expertise.

## Non-Functional Requirements

### Security

- Encrypt data in transit with TLS.
- Encrypt objects at rest through the storage provider or application-managed keys.
- Store passwords with strong modern hashing.
- Support least-privilege service access.
- Audit sensitive actions such as login, sharing, deletion, restore, and permission changes.
- Plan for malware scanning pipeline before public download in later phases.

### Scalability

- Metadata and object storage concerns should be separated.
- API tier should be horizontally scalable.
- Upload/download paths should avoid routing large file payloads through unnecessary layers.
- Background jobs must be retryable and idempotent.

### Reliability

- Design for resumable transfers and recoverable job execution.
- Use checksums and durable metadata writes to avoid corruption.
- Plan backups for metadata and clear disaster recovery procedures.

### Performance

- Optimize large-file transfer paths.
- Use chunking and parallel upload where beneficial.
- Keep directory listing and metadata lookups efficient.
- Limit sync CPU, memory, and disk overhead on client systems.

### Maintainability

- Keep Rust and Java boundaries explicit.
- Prefer contract-based interfaces between services.
- Maintain strong API schemas and event definitions.
- Document data lifecycle and sync behavior clearly.

## Proposed Architecture

### API Layer in Java

Responsibilities:

- expose REST endpoints
- authenticate requests
- validate payloads
- coordinate metadata operations
- issue upload sessions and pre-signed object storage URLs
- manage sharing, permissions, and admin workflows

Suggested bounded contexts or packages (Spring modules or plain Java packages, depending on layout):

- auth
- users
- files
- folders
- uploads
- downloads
- shares
- versions
- admin
- audit

### Filesystem and Sync Layer in Rust

Responsibilities:

- watch local filesystem changes
- normalize OS-specific filesystem behavior
- compute hashes and chunk boundaries
- perform safe local file reads/writes/moves
- manage sync queues and conflict detection
- maintain a local sync index

Possible Rust deliverables:

- a reusable core library for sync logic
- a local daemon/service for background sync
- optional CLI for diagnostics and local operations

### Data Model Overview

Core entities:

- user
- team or tenant
- file
- folder
- object
- file_version
- share_link
- access_grant
- upload_session
- sync_cursor
- audit_event
- background_job

Important design rule:

- metadata records must be distinct from physical object blobs
- object storage keys should be immutable where possible
- file versions should reference object versions rather than overwrite in place

### Recommended Storage Strategy

- Use cloud object storage for binary content.
- Use a relational database for metadata, permissions, jobs, and audit logs.
- Use a cache or message broker for event-driven background processing as scale grows.

Examples of cloud-compatible backends:

- S3-compatible storage
- Azure Blob Storage
- Google Cloud Storage

### API Planning Notes

Representative endpoint groups:

- `POST /auth/register`
- `POST /auth/login`
- `POST /files/upload-sessions`
- `PUT /files/{id}/content`
- `GET /files/{id}`
- `GET /files/{id}/download`
- `POST /folders`
- `GET /folders/{id}/children`
- `POST /shares`
- `GET /files/{id}/versions`
- `POST /files/{id}/restore`

The API should prefer:

- stateless request handling
- idempotent operations where possible
- explicit versioned schemas
- support for pagination, filtering, and auditing

### Sync Planning Notes

The sync engine is one of the highest-risk areas and should be developed early as a vertical slice.

Key design choices to settle before implementation:

- source of truth for conflict detection
- rename and move detection strategy
- local file identity strategy
- chunking and deduplication policy
- offline change queue behavior
- retry and backoff behavior
- handling of partial uploads and interrupted downloads

## Development Plan

### Phase 0: Discovery and Architecture

Deliverables:

- product requirements baseline
- domain glossary
- service boundaries
- architecture decision records
- initial data model
- threat model
- deployment target selection

Open questions to resolve:

- single-tenant vs multi-tenant from the beginning
- direct-to-object-storage uploads vs API-proxied uploads
- database selection
- event bus or queue choice
- target cloud provider
- desktop sync target platforms for first release

### Phase 1: Repository and Foundations

Deliverables:

- monorepo structure
- Java API service skeleton (for example Maven or Gradle)
- Rust workspace skeleton
- shared API contracts and schemas (OpenAPI, protobuf, or similar)
- local development environment
- CI pipeline
- formatting, linting, testing, and release standards

Suggested repo shape:

- `services/api` or `apps/api` for Java REST APIs
- `services/metadata` if split later
- `rust/` or `crates/` for Rust libraries and agents
- `docs/` for ADRs, API contracts, and operational docs
- `infra/` for deployment and provisioning

### Phase 2: Metadata and Object Model MVP

Deliverables:

- folder tree management
- metadata CRUD for files and folders
- audit log foundation
- database migrations

Success criteria:

- the system can manage folders and metadata for storage objects without full identity features

### Phase 3: Upload and Download MVP

Deliverables:

- upload session creation
- direct object storage integration
- checksum validation
- download URLs or proxy download flow
- file metadata finalization after upload
- size and content-type validation

Success criteria:

- users can upload, download, delete, and recover files reliably

### Phase 4: Versioning, Sharing, and Recovery

Deliverables:

- file version history
- recycle bin behavior
- restore operations
- direct share permissions
- share links with expiry and revocation

Success criteria:

- users can safely share content and recover from mistakes

### Phase 5: Rust Sync Agent Vertical Slice

Deliverables:

- local watched folder support
- filesystem event normalization
- local state store
- remote poll or event consumption
- upload of changed local files
- download of changed remote files
- basic conflict handling

Success criteria:

- one desktop environment can synchronize a single workspace reliably

### Phase 6: Cross-Platform Sync Hardening

Deliverables:

- Windows, macOS, and Linux compatibility review
- rename/move detection improvements
- bandwidth controls
- retry and backoff tuning
- resilience against crashes and reconnects
- sync diagnostics and logs

Success criteria:

- sync behavior remains stable under real-world interruption scenarios

### Phase 7: Operations, Production Readiness, and User Management

Deliverables:

- metrics, tracing, dashboards, and alerts
- admin capabilities
- retention jobs
- backup and restore playbooks
- security hardening
- performance and load testing
- authentication and authorization
- user and account model
- role-based access control hardening

Success criteria:

- system can be operated safely in production with full identity and access controls enabled

## Team and Role Planning

Suggested workstreams:

- backend/API team for Java services
- storage and data team for object integration and metadata design
- Rust/sync team for local agent and OS interaction
- platform team for CI/CD, infra, observability, and security

## Major Risks

- sync conflict logic becoming inconsistent across platforms
- metadata drift from object storage state
- poor large-file upload experience
- permission bugs that leak shared content
- versioning and restore edge cases
- operational complexity introduced too early

Mitigations:

- define invariants early
- build end-to-end vertical slices before broad feature expansion
- keep contracts explicit between Rust and Java components
- prioritize auditability and idempotency

## Initial Milestones

1. Approve requirements and architecture direction.
2. Finalize service boundaries and repository layout.
3. Define API contracts and core data model.
4. Build metadata-first MVP.
5. Integrate object storage upload and download.
6. Add sharing and versioning.
7. Deliver Rust-based sync agent prototype.
8. Finalize user/auth management and production hardening.

## Decisions Needed Before Coding

1. Which cloud provider or object storage target should be the primary deployment platform?
2. Should the first release support only personal storage, or also shared team workspaces?
3. Should uploads go directly from client to object storage, or pass through the API first?
4. Which database should hold metadata?
5. Which desktop platform should be the first sync-agent target?
6. Is the first client a web app, a desktop app, or API-only?

## Next Planning Outputs

After this README, the next useful planning documents would be:

- architecture decision records
- domain model diagram
- API contract draft
- sync conflict resolution specification
- deployment topology diagram
- security and threat model
