# Entity Lifecycle in Paros

**English** · [Français](../fr/cycle-de-vie.md)

This document describes the lifecycle, physics, and state transitions of adult mascots, subagents, the Sage advisor, and flying letters.

---

## 1. Adult Mascots (`Pet`)

Each running session (Claude Code or Antigravity) instantiates a unique adult mascot.

```
       Session Opened
              │
              ▼
          [Spawn] ──────(ground)────▶ [Walk] ◀────────┐
              │                         │             │
        (window nearby)           (idle timer)  (user task)
              │                         │             │
              ▼                         ▼             │
          [Perched]                  [Idle] ──────────┘
      (active window)                   │
              │                 (long inactivity)
              │                         ▼
              │                       [Sit]
              ▼
    [Carried / Dragged] ────(thrown)───▶ [Ballistic Fall] ───(ground)───▶ [Impact / Thud]
```

### Key States
- **`Idle`**: Mascot stays in place, blinks, watches mouse cursor when nearby. Lights a campfire when the session is completely resting.
- **`Walk`**: Autonomous ground locomotion with waddling physics (`leg_phase`). Pace is modulated by system load and baggage burden (`pace`).
- **`Perched`**: Jumps onto the top border of the active Hyprland window. Falls if the window moves or closes.
- **`Carried`**: Picked up by mouse cursor during drag & drop.
- **`Fall`**: Free-fall with gravity (`vy += 980.0 * dt`). Deploys an umbrella if fall height exceeds 200 px. Emits a dull thud upon landing.
- **`Throw` / `Read`**: Poses for sending and reading inter-session flying letters.

---

## 2. Subagents (`small_pets`)

When a session launches subagents via `invoke_subagent` (Antigravity) or `Agent` / `Task` (Claude Code):

1. **Birth (`sync`)**:
   - Spawns immediately at the parent's feet.
   - Sprints outward (`BIRTH_RUN` 60–130 px at `pace = 2.2`) holding a sheet of paper.
2. **Orbital Reach (`home_reach`)**:
   - Bound to the parent via `parent_id`.
   - Wander zone (`home_reach = 140 + 30 * count`) dynamically follows the adult mascot.
3. **Playful Behaviors**:
   - Joyful hops.
   - Visits between sibling subagents.
   - Collective multi-tier acrobatics pyramids.
4. **Departure (`_dismiss`)**:
   - Once completed, performs a cheer jump, tucks its paper away, and sprints back to the parent.
   - Despawns upon reaching the parent or after a 6-second timeout.

---

## 3. Sage Advisor (`new_sage`)

When a session triggers an advisor consultation:

1. **Arrival**:
   - A venerable Sage (1.5× stature, pointy hat, beard, walking staff) enters from the closest screen edge.
   - Sprints (`pace = 2.0`) to a halt ~230 px from the parent mascot.
2. **Correspondence**:
   - Parent mascot turns and launches a flying letter (`FlyingLetter`).
   - The Sage reads the letter (`State::Read`).
3. **Departure**:
   - When the advisor response arrives, the Sage tosses a return letter back.
   - Waves farewell, walks off-screen, and is cleaned up from memory.

---

## 4. Flying Letters (`FlyingLetter`)

Letters visualize inter-session communications and advisor queries:

- **Ballistic Trajectory**: Dynamic parabolic arc (`arc = min(distance * 0.25, 220.0)`).
- **Angular Rocking**: The envelope gently tilts during flight (`angle = sin(time * 9.0) * 0.18`).
- **Colored Wax Seal**: Matches sender mascot color (Claude orange `#d97757` or Agy blue `#4a80ea`).
- **Landing**:
  - Held and read immediately if the recipient is ready.
  - Stored in the recipient's mailbox (`mailbox`) with an unread badge counter if busy.
