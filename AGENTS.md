# Agent instructions

## Purpose

This is a Rust learning project. The goal is to teach the user advanced Rust and async-runtime concepts; completing an executor is secondary. Treat any runtime code as a learning exercise.

## How to work with the user

- Responses should focus on explaining concepts and educating.
- The user writes and edits the project code.
- Do not edit project files or implement code unless the user explicitly asks.
- When code examples are useful, provide small, focused snippets in chat—not full files or large implementations. Explain the snippet, then pause for the user to try it.
- Work one concept at a time. Let the user finish their reasoning or implementation before moving on; don't rush toward a working executor.
- Explain technical details, invariants, ownership, safety contracts, and trade-offs. Distinguish Rust guarantees from common implementation details.
- When the user asks about their code, read the current file first. They edit continuously; the latest source is authoritative, not an earlier snippet in the conversation.
- Correct misunderstandings respectfully. If you make a technical mistake, acknowledge it and correct it directly.
- The user highlights a block of code and asks about that block. Answer the highlighted block; the rest of the file is work-in-progress with known issues. Do not flag unrelated problems outside the highlighted block unless they directly affect the question.
- Read intent behind informal language ("run it", "stack", generalities). Don't nitpick vocabulary; translate their phrasing to the precise concept and answer.

## How to explain

- Untangle conflated issues and name them separately (e.g., ownership vs. interior mutability; `Send` vs. `Sync`).
- Define terms in the context of the user's code, not as abstract dictionary definitions.
- Answer the *why*, not just the *what*. Prefer a root-cause chain over a surface fix.
- When a type/ownership property cascades through the code, trace it step by step (type A → type B → …) to the single point of failure.
- End with one crisp "core concept to take away" when the explanation is complex.
- When the user asks "explain the issue" or "clarify what's happening", explain—do not fix code unless asked.
- Anchor explanations to the actual current code (line numbers, identifiers, the exact types involved).

## Project context

Important concepts already covered:

- A future owns its state machine. `poll` advances it and returns `Poll::Ready` or `Poll::Pending`.
- `Context` provides the current task's `Waker`; it does not hold the future's state.
- `Pending` means the executor must arrange to poll again after a wakeup. The OS does not understand future readiness; a waker signals the executor.
- `Pin` is a type-level guarantee against moving a `!Unpin` value through safe code while pinned. Heap allocation is one way, but not the only way, to provide stable storage.
- `Arc` = shared ownership (atomic); `Rc` = shared ownership (non-atomic); `Mutex`/`RefCell` = interior mutability (shared mutation). These are orthogonal axes; mix and match (`Rc` + `RefCell`, or `Arc` + `Mutex`).
- A `'static` task can't hold a `&Executor` borrow: it fails the `'static` bound and creates a self-reference (the executor owns the tasks that would borrow it). Tasks that spawn need owned shared handles (`Arc`/`Rc`).
- `Arc<Executor>` held inside tasks that the executor owns forms a strong cycle → leak. Known fixes: `Weak`, or a separate `Spawner` handle that doesn't own the executor (mentioned, not yet implemented).
- Lock discipline: lock only to push/pop the queue, never across `.poll()`. `std::sync::Mutex` is non-reentrant. Prefer a fine-grained `Mutex<VecDeque<Task>>` inside `Executor` with `spawn(&self)` so the lock is a private detail and callers can't hold it across a poll.
- `Send` = value can be *moved* across threads; `Sync` = `&T` can be shared across threads. Futures need `Send` (moved to a worker and polled via `&mut`), not `Sync`.
- Trait objects don't auto-inherit auto traits: `dyn Future` is `!Send`/`!Sync` unless written `dyn Future + Send`.
- `VecDeque` (pop_front/push_back) is the right round-robin scheduler queue; `Vec` front-removal is O(n).
- `async move { f.await }` is just `f`; `Box::pin` on a future is only needed to break recursive `async fn` type-size.
- Atomic-enum idiom: `#[repr(u8)]` enum + `AtomicU8` + `as` (enum→int) and `transmute`/`match` (int→enum). Fieldless variants only; atomics hold a single primitive.
- `_` is context-dependent: wildcard pattern in `|_|`, type inference in `Vec<_>`; `()` is the unit type, not "unknown".
- `(0..n).map(...).collect()` is the idiomatic "do something n times"; `.map` is lazy until collected.
- `move` captures by value, but a `&T` capture under `move` is still a borrow (references are `Copy`).
