# Feature Support for Encrypted Files

This document outlines the best practices and architectural patterns for handling conflicts in encrypted file syncs across multiple clients.

Resolving conflicts in encrypted files across multiple clients is one of the trickiest problems in distributed systems. Because the files are encrypted, the central syncing server operates with "zero knowledge"—it cannot see the file contents, which means it cannot perform automatic, server-side merging (like Git does for plain text).

Therefore, the golden rule of encrypted sync is: **All conflict resolution must happen on the client side.**

Here are the best practices and architectural patterns for handling conflicts in encrypted file syncs across multiple clients.

---

### 1. Robust Conflict Detection (Metadata)

Before you can resolve a conflict, the system needs to accurately detect that a concurrent modification happened. Because the server can't look at the payload, it must rely on metadata.

* **Use Vector Clocks or Version Vectors:** Relying purely on timestamps (e.g., "last modified") is dangerous due to clock drift across different clients. Vector clocks track the sequence of updates per client (e.g., `[ClientA: 2, ClientB: 1]`). If the server receives a file with a vector clock that isn't a strict descendant of the server's current version, a conflict is flagged.
* **File Hashing:** Always include a hash (like SHA-256) of the *unencrypted* file in the encrypted metadata payload. This allows the client to quickly determine if the conflicting files actually contain the exact same data, avoiding unnecessary manual resolution.

### 2. The "Side-by-Side" Forking Strategy (Prevent Data Loss)

When the server detects a conflict, it should **never overwrite** existing data or attempt to arbitrarily pick a winner based on timestamps (which leads to the infamous "silent data loss").

* **Fork the File:** The server should accept the upload but save it as a distinct, parallel version.
* **Rename for Clarity:** Append a suffix to the filename indicating the conflict. For example, if Client B uploads a conflicting `budget.xlsx.enc`, the server saves it and propagates it to all clients as `budget (Client B's conflicted copy).xlsx.enc`.
* **Leave it to the User:** For binary files (images, PDFs, compiled documents), automatic merging is impossible anyway. Forking forces the user to manually open both files, decide which one is correct, delete the loser, and rename the winner back to the original filename.

### 3. Client-Side Decryption and Merging (Active Resolution)

If your application syncs files that *can* be merged (like plain text, JSON, or SQLite databases used by your app), you can build an automatic resolution pipeline into the client application.

1. **Download:** The client downloads both the original encrypted file and the conflicted encrypted file from the remote server.
2. **Decrypt:** The client decrypts both files locally in memory.
3. **Merge:** The client application uses a standard diff/merge algorithm (or Conflict-free Replicated Data Types - CRDTs) to combine the changes.
4. **Re-encrypt & Upload:** The client encrypts the newly merged file, increments the version vector, and uploads it back to the server, overwriting the old forks.

### 4. Granular / Chunk-Based Syncing

Syncing monolithic encrypted files increases the surface area for conflicts. If two users edit entirely different parts of a 100MB video project file, a full-file sync creates a massive conflict.

* **Chunking:** Split large files into smaller, individually encrypted chunks (e.g., 1MB blocks). Keep a "manifest" file that maps the file structure to these encrypted chunks.
* **Benefits:** If Client A modifies chunk 3 and Client B modifies chunk 85, they can both upload their respective chunks without conflicting. Conflicts only occur if both clients modify the *exact same chunk*.

### 5. Preventative Measures: Pessimistic Locking

If your system assumes users will mostly be online, you can reduce conflicts before they happen using file locks.

* **Advisory Locks:** When a user opens a file, Client A sends a "lock" request to the server. If Client B tries to open the file, the server informs Client B that it is locked by Client A, and opens it in read-only mode.
* **Caveat:** This falls apart if users go offline, edit the file, and then come back online. You still need the "Side-by-Side" forking strategy as a fallback.

---

**Summary Checklist for Implementation:**

* Server does **not** touch or merge data.
* Use **version vectors** to detect concurrent edits.
* Default to **forking the file** (saving both copies) to prevent silent data loss.
* Perform all decryption and data merging **locally on the client**.