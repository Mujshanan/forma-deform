# Smart Bones Product Requirements

- **Status:** Draft for architecture review
- **Product objective:** Add Moho-style Smart Bones functionality to AniCraft
- **Related architecture:** FDM-ARCH-002 (accepted direction); ADM-001 through ADM-007 remain unresolved
- **Scope:** Product behavior and MVP requirements; this document does not authorize implementation

## 1. Purpose

AniCraft Smart Bones should let an animator use a bone as a control dial for authored corrective animation. As the control bone rotates, AniCraft evaluates the associated corrective action and blends its keyed changes into the character pose.

This is more than a bone-angle-to-number mapping and more than a one-time pin edit. It is an authored animation relationship between a bone control, an action, and one or more affected animation channels.

Moho's documented workflow provides the reference model: a named bone is associated with a Smart Bone Action; the control's minimum and maximum angles define its range; the animator authors changes inside the action; the action is driven by the bone rotation. Moho also supports separate positive- and negative-angle actions. See the Moho 13.5 User Manual, "Creating a Smart Bone Action" (p. 105) and "Make Smart Bone Dial" (pp. 464–465):
- https://lostmarble.net/Manuals/Moho%20Users%20Manual_HQ.pdf
- https://www.lostmarble.com/manual/13.5/Moho%20Users%20Manual.pdf

The goal is comparable animator-facing functionality, not a clone of Moho's internal implementation.

## 2. Core user workflow

1. Create or select a bone that will act as a Smart Bone control.
2. Create a Smart Bone Action associated with that control.
3. Configure its neutral, minimum, and maximum control angles, including the direction/range of influence.
4. Enter the action-authoring mode.
5. Pose the control and author corrective changes to one or more supported targets at action positions.
6. Return to the main animation.
7. Rotate the control bone. AniCraft evaluates the action at the corresponding action position and combines the result with the ordinary character pose.
8. Scrub and play the timeline; the same control/action relationship must evaluate consistently.
9. Save, close, and reopen the document; the Smart Bone and its authored action must remain intact.

The animator should not have to manually run a plug-in command for every frame.

## 3. Product concepts

| Concept | Meaning |
|---|---|
| Smart Bone control | A stable reference to a bone whose evaluated rotation drives an action |
| Neutral angle | The control angle at which the action contributes its neutral state |
| Control range | The supported angular interval or intervals and their direction |
| Smart Bone Action | Authored, time/key-based corrective animation associated with a control bone |
| Action progress | The normalized position within the action derived from the control angle |
| Target channel | An animatable property or geometry channel changed by the action |
| Evaluated correction | The deterministic contribution of the action at the current control angle |
| Main pose | The ordinary AniCraft animation evaluated without the Smart Bone correction |

A Smart Bone Action is not the same thing as AniCraft frame-script/event actions. The product needs an animation-action concept that can contain authored keyframes for supported target channels.

## 4. Functional requirements

### SB-REQ-001 — Bone association
A Smart Bone Action MUST refer to a stable bone identity, not a bone's current list position or display name alone. Renaming a bone must not silently detach its action. Missing or deleted control bones must produce a detectable invalid-reference state.

### SB-REQ-002 — Angular control
The system MUST derive action progress from the evaluated control-bone angle. The control definition must identify the neutral angle and supported angular range. Range direction, angle wrapping, constraints, and treatment of spring swing must be explicit.

### SB-REQ-003 — Authored corrective action
The animator MUST be able to author changes inside an action over an action timeline. The action must contain more than a single endpoint pose, allowing intermediate corrections where needed.

### SB-REQ-004 — Multiple target channels
An action SHOULD be able to animate more than one supported target channel. Candidate channels include bone pose, artwork geometry, warp pins, and layer/object properties. The initial supported set is a product decision and must be verified against AniCraft's model and evaluator.

### SB-REQ-005 — Progress mapping and interpolation
The system MUST map the control angle deterministically to action progress. It must define behavior at neutral, both range endpoints, out-of-range values, reversed ranges, and intermediate values. Interpolation must respect the action's authored keyframe semantics.

### SB-REQ-006 — Mainline integration
Smart Bone corrections MUST be evaluated as part of the character pose, not applied as repeated destructive edits to the current document state. The same immutable authored inputs and pose inputs must produce the same result.

### SB-REQ-007 — Preview, scrubbing, and playback
The animator MUST see the correction while posing the control, scrubbing the main timeline, and playing the animation. Preview, timeline evaluation, and rendered/exported output must agree.

### SB-REQ-008 — Persistence
Smart Bone definitions, control references, range settings, action tracks, and authored keys MUST survive save, close, and reopen. The format must be versioned and support validation/migration.

### SB-REQ-009 — Undo and redo
Creating, editing, renaming, reassigning, and deleting Smart Bone controls/actions must participate in AniCraft's undo/redo model. Previewing must not leave uncommitted document mutations.

### SB-REQ-010 — Multiple controls
The design MUST allow a character to have multiple Smart Bone controls. The MVP may test one control, but data ownership and evaluation must not assume one global control.

### SB-REQ-011 — Deterministic composition
When multiple actions affect the same target, their interaction must be explicitly defined. The system must not assume additive, override, priority, or order-dependent composition without a decision.

### SB-REQ-012 — Export and baking
The product SHOULD provide a path to render or bake the evaluated result into ordinary animation/export data where required. Baking representation and loss of editability are separate decisions and are not an MVP prerequisite.

## 5. MVP proposal

The first vertical slice should prove the complete control-to-render workflow with the smallest practical feature set:

- One Smart Bone control on one rig.
- One associated Smart Bone Action.
- A neutral state and authored corrections at the two ends of a defined angular range.
- At least one intermediate action key to verify interpolation.
- One target geometry channel supported by AniCraft (candidate: existing warp-mesh pin data, subject to host-evaluator design).
- Evaluation from the current bone pose without destructive accumulation.
- Correct result during interactive posing, timeline scrubbing, and playback.
- Save/reopen persistence.
- Undo/redo for authoring changes.

The MVP should not require lattice deformation, nested deformer stacks, multiple-source composition, or baking. Those capabilities should remain architecturally possible but must not delay proof of the core Smart Bone workflow.

## 6. Acceptance scenarios

1. **Neutral:** At the neutral control angle, the action contributes its neutral state and does not displace the uncorrected pose.
2. **Positive endpoint:** At the configured positive endpoint, the authored positive correction is reproduced.
3. **Negative endpoint:** If a negative range/action is configured, its endpoint correction is reproduced independently.
4. **Intermediate:** At an angle halfway through a linear range, the result matches the action's defined interpolation at halfway progress.
5. **Repeatability:** Re-evaluating the same frame and bone pose repeatedly produces identical output; no deformation accumulates.
6. **Timeline:** Scrubbing away and back to a frame reproduces the same result.
7. **Playback:** Playback renders the same result as direct evaluation at the same frame.
8. **Persistence:** Save/reopen preserves the control, action, target references, and keys.
9. **Undo:** Undoing an authored correction restores the prior action state.
10. **Invalid reference:** Removing the controlling bone or target does not silently redirect the action to another item.
11. **Multiple controls (post-MVP):** Two controls can be evaluated without assuming an undefined composition rule when their target sets overlap.

## 7. Explicitly unresolved product questions

- Does one control own one action, or can it own distinct positive and negative actions and additional ranges?
- Is the neutral angle always zero, or may it be arbitrary?
- Is angle input the keyed constrained angle, the spring-adjusted angle, or another evaluated pose value?
- Which target channels are included in the first release?
- Are actions stored as tracks inside the deformation document, the AniCraft document, or a dedicated animation-action model?
- How do corrections combine with ordinary animation and with other Smart Bones?
- What happens when controls are parented, constrained, mirrored, or nested in symbols?
- What is the expected behavior for full rotations and angle wrapping?
- What is the baking/export contract?

## 8. Non-goals for the first vertical slice

- Recreating every Moho feature or matching Moho's internal data model.
- Implementing general lattice or nested-deformer authoring before the Smart Bone workflow is proven.
- Treating an MCP-driven sequence of commands as equivalent to native playback evaluation.
- Applying corrections destructively to the current pose on every frame.
- Defining multi-source composition by incidental execution order.

## 9. Authority and change control

This is a draft requirements artifact. It does not resolve ADM-001 through ADM-007, amend FDM-ARCH-002, authorize changes to the AniCraft repository, or authorize merging any existing pull request. Requirements become accepted only through explicit review and approval.
