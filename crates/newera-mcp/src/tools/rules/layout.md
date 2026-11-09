# Layout: what each problem means

`layout` answers the problems by kind; `{}` means none. Every row is an object with
the `key` it is accepted by and the elements involved (name, bounds and z).

- **overlap**: classified `collision` (a real clash, listed first), `nesting` (built in,
  resting on, tucked under), `served` (a project point inside a piece on purpose: the water
  point in the basin, the outlet behind the fridge or set into a cabinet) or `cross_level`
  (pieces of two storeys at the same height in the building: storeys drawn at one elevation,
  or a piece reaching into the storey above), with extent [x,y,z] cm of the shared space;
  overlap_kinds counts them.
- **blocked**: a cabinet, fridge or wardrobe whose opening face is against a solid. It
  cannot be used, and `angle` alone does not show it.
- **opens_into** `{key, piece, front, against, how}`: a door, drawer or appliance door
  (`how`: `door`, `porthole`, `drawer`, `flap_down`, `flap_up`) that runs into a solid while it opens —
  the wall beside its hinge, the fridge next to it, the handle of the drawer in the corner, the
  box hung over a wall cabinet's lift-up flap. Closed, nothing touches, so no other check sees
  it. A door sweeps its width around the hinge to 90° and stands past the hinge side by its
  thickness and handle (a fridge, freezer or microwave swings its door like this); a washer's
  porthole (`opens: porthole-left|porthole-right`) turns about a hinge 4 cm off its face to 135°,
  about 45 cm out and 30 cm past its hinge side, so it clears a niche 4 cm deep but not a wall or
  tall cabinet right beside it; a drawer comes out by its slide; an oven, dishwasher or dryer door
  drops flat at its bottom edge and a lift-up flap (`opens: flap-up`) rises flat at its top
  edge, each reaching out by its height (an appliance's door, not its plinth and panel: about
  70 cm of a dryer 85 cm tall). A filler panel at the corner, the hinge on the other
  side, a narrower handle or more room in front clears it. A flap that lifts past the ceiling
  meets the room (`against` is the room's id): hang the cabinet lower or give it hinged doors.
- **in_wall** `{key, piece, wall}`: a piece inside a wall.
- **blocks_door** `{key, door, by}`: includes a 60 cm approach on either face, even for
  sliding doors and passages.
- **blocks_window**: nearby tall or elevated solids masking the window, with extent
  [width,height] cm; compact countertop objects, plants, lights, glass and guards (bar
  railings, glass balustrades, balcony glazing) are exempt, so this does not certify sash
  operation, ventilation or protection against falls.
- **no_door**: a bedroom or bathroom (by name) with no door — only open passages, listed,
  or no way in at all — said once the storey has doors somewhere; a living room, kitchen or
  balcony left open is not.
- **backwards**: a piece whose back belongs on a wall (sofa, bed, toilet, TV, desk — the
  catalog marks them `wall`) with its front against a wall instead: turned half around.
  fix.angle turns it the right way; its back then touches that wall only if it was flush —
  placing again with `wall=<id>` seats it.
- **turned**: a group whose built fronts (doors, drawer fronts, kick) face one way and whose
  `angle` says another: the piece opens where the panels are, so fix the angle, not the
  clearance it seems to lack.
- **unrated_light**: a light fixture with neither lumens nor watts — only the relative power
  an import carries — so the lighting review's lux for its room are a guess: set its output
  with update(light={lm or w}).
- **unclear_front**: a group whose parts name fronts on more than one face with no clear
  winner (candidates, strongest first; placed is the angle's guess every "in front of"
  falls back to). Rename the misleading part or set the angle. Handles weigh most.
- **loose_opening**: a door or window in no wall — a passage drawn as a panel — which reads
  as an opening in every schedule and opens nothing.
- **outgrew_niche**: an appliance its host stopped holding after the joinery was resized
  around it, with how far it sticks out. Built-in pieces are left out of the overlap check
  by design, which is why nothing else notices.
- **outside_rooms**: a piece outside every room; a group wholly outside (a facade panel of
  slats) is one row, and a part of one sticking out is that part.
- **loose**: a fixed point with nothing to be fixed to — loose in a room, on glass, in a
  door or window span, hanging under the ceiling — with why.
- **above_ceiling**: compares the full luminaire footprint with the room ceiling surface the
  3D uses, with ceiling/top/over in cm: declared storey height, sloping wall profiles and
  lower roof panels. Hidden ceilings and unknown uncovered slopes are not inferred.

## Accepting

`accept` marks a finding looked at and right as drawn — an imported model whose box is
bigger than the piece it draws. It leaves the sections, the variant count and every dry
run, and is listed under `accepted {key, kind, why, extent}` with its reason, kept in the
project. `orphaned [[key, reason]]` lists acceptances whose finding is gone on every storey;
`accept(prune=true)` drops them.
