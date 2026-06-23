# Decisions — 002-poll-loop-conversion

## Reviewer gate

reviewer-1 was unresponsive after two messages. Proceeded to implementation after the maximum wait (per protocol: "proceed regardless" after one fix cycle). No review notes received.

## Clippy: collapsed nested if guards

Clippy flagged two `collapsible_if` warnings in the poll-based loop. Applied the suggested collapse using `&&`-chained `let` guards (`if event::poll(FRAME_TIME)? && let Event::Key(key) = event::read()? && key.kind == KeyEventKind::Press`). This is idiomatic Rust and satisfies the spec's "cargo clippy is clean" requirement. The positive-condition structure still correctly ensures non-Press events fall through to `app.tick(dt)`.

## Loop order: draw before dt computation

The spec's Technical Approach lists the loop steps as: draw → compute dt → poll → tick. This was followed exactly. dt therefore includes draw time, which is acceptable (spec explicitly defers dt capping to a future spec).

## break Ok(()) vs return Ok(())

Replaced `break Ok(())` with `return Ok(())` for the Quit arm — semantically equivalent, and slightly cleaner with the new poll-based structure where `break` would need to carry a value out of the loop.
