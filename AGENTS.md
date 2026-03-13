# AGENTS.md

## Purpose

This repo is for building Backlayer: a Hyprland-first animated wallpaper runtime with a focused MVP.

## Working Rules

- Keep the MVP narrow: `Hyprland`, `wlroots` layer-shell, video, shader, image, daemon, simple UI.
- Do not expand scope to other desktop environments during MVP work unless explicitly asked.
- Treat `PROJECT_SUMMARY.md` as the source of truth for product direction.
- Treat `TODO.md` as the source of truth for execution status.

## Required Behavior While Working

- Before making substantial changes, check `TODO.md` and work from it.
- When starting a meaningful task, update `TODO.md` if a missing task should exist there.
- When finishing a task, check it off in `TODO.md` in the same change set when practical.
- If scope changes, update both `PROJECT_SUMMARY.md` and `TODO.md` together.
- Prefer adding new tasks rather than keeping important work only in memory.
- Keep docs aligned with reality; do not leave completed architecture decisions undocumented.

## Engineering Reminders

- Prioritize a working layer-shell background surface early.
- Ensure wallpaper surfaces ignore input.
- Design around per-monitor lifecycle management from the start.
- Keep renderer backends separated by responsibility.
- Build daemon ownership of config, process lifecycle, and recovery instead of scattering that logic into the UI.
- Add performance controls early enough that the MVP does not feel irresponsible on laptops.

## Definition Of Progress

Progress means shipped, checkable outcomes:

- a task exists in `TODO.md`
- the implementation matches the task
- the task gets checked off when done
- follow-up work is captured as new unchecked tasks

## Anti-Patterns

- Do not let the repo drift into vague planning with no checked tasks.
- Do not silently change scope.
- Do not build optional polish before the core runtime works.
- Do not treat the UI as the product; the runtime is the product.
