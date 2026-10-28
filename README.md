# melba
> "Perfection is achieved, not when there is nothing more to add, but when there is nothing left to take away."
>
> — Antoine de Saint-Exupéry


A tiny single-threaded async runtime in Rust, named after a cat who spends most of her life asleep.

```
      z
    z
   /\_/\
  ( -.- )   zzZ
   > ^ <
```

`melba` is a from-scratch async executor I built as a learning exercise. Its entire public API is two functions:

- `spawn(future)` — hand the runtime a task to run.
- `sleep(duration)` — pause the current task for a while.

## Usage

```rust
use melba::{sleep, spawn};
use std::time::Duration;

fn main() {
    spawn(async {
        for i in 0..3 {
            println!("work, work, work... ({i})");
            sleep(Duration::from_millis(500)).await;
        }
    });
}
```

The first `spawn` drives the entire run loop to completion, so `main` needs nothing more than that one call.

## Development

Run the tests with:

```sh
cargo test -p melba
```

## Details

This was an exercise to strengthen some of my Rust skills but also to figure out a better way to work with AI.

I tried a few things at first to get the AI to teach me rather than fire a pile of crap out. At first I had gone back and forth in 'planning' mode and came up with a whole learning workflow, with it going through things slowly and trying to be an actual teacher. But this ended up with it just feeding me snippets of code and waiting for me to type them into and editor and not explaining anything, plus the 'work' was boring and I wasn't actually learning anything.

In the end, I found just a few guardrails in an AGENTS.md file and just building on my own and asking it questions was the sweet spot. I could make mistakes and get stuck and then ask for help or an explanation without it just blasting the answer or it getting stuck in a "teaching loop". I've been finding that 'less is more' when it comes to what to write in an AGENTS.md file or a skill file - too many constraints and it ends up fretting forever about breaking one, so be specific, but not constraining.

Another big takeaway is to stay engaged. I found that the more I kept a mental model of what was going on, was actively writing code, and was _in_charge_, the longer I was able to work. As soon as I let the AI start steering, after an hour or so I ended up just letting it write more and more code and pretty soon I wasn't learning anything at all, just prompting it and waiting for a reply. 

I think that's the trick for actually producing good code in the long run with AI, don't try and write as many lines of code as you can, you need to keep a mental model and for me personally, I need to be engaged, which means writing _some_ actual code.
