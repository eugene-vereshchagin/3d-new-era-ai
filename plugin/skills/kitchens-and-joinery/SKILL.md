---
name: kitchens-and-joinery
description: Design kitchens, wardrobes and custom joinery in 3D New Era AI — cabinet runs sized to the wall, appliances embedded, slatted panels, countertops — and produce the cut list. Use for kitchens, closets, built-ins and anything a joiner will build.
---

# Kitchens and joinery

1. **Fill a wall.** `cabinet_run` with `wall=<id>` measures the free stretches between
   corners, doors, windows, fridge and stove and splits them into even modules
   (`dry=true` shows the plan without drawing). `p.sink` and `p.cooktop` place those
   cabinets and their cutouts. On a wall row `p.lift=true` hangs lift-up flaps instead of
   hinged doors, over the fridge too; a blind corner keeps its hinged door. A single
   `joinery` cabinet takes `door: lift`. `p.board: "MDP"` cuts the carcass from
   particleboard, `p.front_board: "MDF"` with `p.front_t: 19` the fronts and fillers from
   their own board: the cut list then gives each its rows and sheets.
2. **Appliances.** `embed` sets a sink bowl or cooktop into a countertop with an exact
   cutout, or an oven or microwave into a cabinet niche; the item then moves with it.
3. **One-off pieces.** `joinery` builds cabinets, slatted panels, countertops, plaster
   coves and sofas from parameters; workshop rules come back as notes and never refuse to
   draw. Change a build later with its `id` and only the new values.
4. **Check.** `layout` finds cabinets whose doors open against a wall, doors, drawers and
   appliance doors that hit a wall, a handle or the fridge while they open, including oven,
   dishwasher and dryer doors that drop and lift-up flaps that rise into the ceiling
   (`opens_into`: a corner filler, the hinge on the other side or more room in front clears it), and appliances a resized niche
   no longer holds; `ergonomics` checks counter heights and the work
   triangle.
5. **Cut list.** `cut_list` reads boards, edge banding and hardware; `export` with `what=cut_list`
   writes it as `.csv`, or `.dxf`/`.svg` sheets for the workshop. Give the workshop's own stock
   where it differs: `sheets` by board name, the longest name start winning. The defaults are
   `*` 2750 × 1830, `Pedra` 3200 × 1900, `Gesso` 2400 × 1200 and `Ripa` 3000 × 300 mm, so
   `{"*":[2800,2070]}` leaves stone on its slab; give `Pedra` too for other slabs. Also `kerf`
   (default 4 mm) and `trim` (default 0). Boards in `oversize` need a splice or a bigger sheet.
