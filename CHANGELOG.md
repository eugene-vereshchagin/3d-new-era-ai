# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org).

## [Unreleased]

### Fixed

- The Linux tarball opens on Ubuntu 22.04, Debian 12, Fedora 36 and newer: it is built on the oldest
  Ubuntu GitHub runs (glibc 2.35) instead of the newest. The 3.2.0 one asked for glibc 2.43 and
  failed to start on Ubuntu 24.04 with `GLIBC_2.43 not found`.
- In English, Spanish and French the editor no longer shows its own defaults in Portuguese: the
  first tab reads "Version 1" (not "Versão 1"), a new project "New home", and the list of
  pieces names catalog items in the window's language ("French door", not "Porta-janela").
  The file keeps what it held; a name somebody typed is shown as typed.
- A shared link to the browser editor (`3dneweraai.com/app/`) shows a card with a title, a line
  about it and a picture of the editor, instead of a bare address. So do the viewer, the
  connector, pricing and the legal pages; a test keeps every public page that way.

## [3.2.0] - 2026-10-27

### Added

- A Microsoft Store package: each release also carries `newera-windows-x64.msix`, the package
  the Store signs and publishes, with `newera` on the PATH for MCP clients and `.newera` files
  opening in the app; the release sends it to the Store when its credentials are set.
- The desktop app says when a newer release is out: at launch it asks GitHub for the latest
  release, and the status bar links to it ("Nova versão 3.2.0 disponível"). Installed copies kept
  running old versions without knowing.
- At every launch, on the desktop and in the web editor, a dialog asks for support: it explains
  that the AIs using the tool send more reports and suggestions than there is time and money to
  fix, links to Buy Me a Coffee and invites issues and pull requests on GitHub.
- `update` takes `into` on a door, the same point `place` takes: the door turns to swing into that
  side, hinged on the same jamb. A door placed opening the wrong way had to be deleted and placed
  again.

### Changed

- Crash reports from the released builds now name the function and line where the program
  crashed. Each release uploads its debug files to Sentry; the downloads stay stripped and the
  same size.

### Fixed

- A paste with nothing readable on the clipboard is really no longer reported as an error: the
  clipboard writes through the `log` crate, and the check saw every such line under the target
  `log` instead of its own. An MCP client that hangs up before its answer is sent, and a Vulkan
  driver that fails to start while wgpu tries the next backend, are no longer reported either;
  all three stay as breadcrumbs.
- Hiding a roof no longer refits the walls and panels fitted to it: they were cut and split
  as if the roof were gone. Hiding is a display change; editing or deleting the roof still
  refits them.
- A render (`render_plan`, `render_3d`, `render_photo`, `edit_video`) is no longer cancelled
  because the client did not answer a ping within 10 s. Pings are still sent to keep the session
  alive, but a render stops only when the client cancels it, disconnects, or a photo passes its
  `max_s`.
- `cabinets` with a `sink_w` under 60 or a `cooktop_w` under 58 no longer closes the program:
  the cutout was kept 30 cm (29 for the cooktop) from each side of a cabinet narrower than that.
  It now goes to the middle of such a cabinet.
- `embed z=` into a cabinet hung on the wall is the height above the room floor, as the tool
  says: a microwave asked at 160 cm in a cabinet at 150 cm sat 160 cm above the cabinet's base,
  past its top. The item now sits on the niche that is built, and the reply's `bottom` is that
  height above the floor.
- `place` with `wall` and `at` but no `along` puts the piece at the point projected onto the wall,
  kept far enough from the ends for it to fit. It used to ignore `at` and centre the piece on the
  wall, so a door asked for near a corner came out in the middle.
- `update h_end`, `update level` and `update opacity` are said as changes. The compact rows a
  write is compared on had no wall height at its end, no storey and no piece opacity, so these
  writes were applied but answered as if the element already had the values. Reads now show
  `h_end` on a sloping wall and `opacity` on a piece that has one, and a move to another storey
  is named as `level` in the reply.
- `place` with `defaults={unit:...}` (or `stretch`) no longer hands them to the catalog items in the
  batch. Every catalog piece was refused with `unit and stretch are for model=<file>`; now only the
  items that import a model take them, and an item that sets them itself is still told.
- `fit_roof` fits a wall to the roofs over its own storey, whichever storey is selected. A
  double-height wall on the ground floor under a roof drawn on the floor above came out 245 cm
  too low — the roof's heights from the upper floor were applied from the ground floor — and
  with the ground floor selected it found no roof at all.
- A `create` that welds wall ends gives the ids it created as they are in `structuredContent`.
  The `welded=` list after them was read as more ids (`"w2.a→[965.0"`, `"580.0]"`), so a client
  reading the fields got ids that do not exist. `welded` is now its own list, one entry per end.
- A light's `z` outside 0..1 in `place` or `update` is refused with "z is a fraction of the piece's
  height (0 = bottom face), not cm". A height in cm such as `z:268.5` was clamped to 1 without a
  word, and the source landed on the top face of the fixture.
- A `render_plan` or `render_3d` asked for while another render runs waits for its turn instead of
  being refused with "A render is already running"; an agent drawing two views at once got one
  and an error. `render_photo` and `edit_video`, which take minutes, are still refused while
  another render runs, and the descriptions say one render runs at a time.
- A scene too big for one GPU buffer (over 256 MiB of vertices or indices: a large project with
  detailed models) no longer closes the editor. The device asks for the largest buffer the GPU
  takes, and a scene larger than that goes up and is drawn in parts.
- `layout` no longer says a roof slope blocks a window or door under it when the slope passes
  above the opening's head: a tilted plate was judged by its whole height range, from eave to
  ridge, instead of where it actually is over the opening.
- Moving a wall takes its doors and windows with it. They stayed where the wall had been,
  standing in the air, and a door moved together with its wall was pulled back to the old
  place. `update` of a wall's `a`/`b` keeps each opening at the same distance from the start,
  inside the wall; one that no longer fits is left where it was and named in the reply
  (`openings_left=`).
- An imported model with a side under 1 cm keeps it: a shelf 8 mm thick comes in 0.8 cm deep. Every
  side was raised to at least 1 cm, so thin panels, plates and knobs grew; only a side with no
  thickness at all (a plane) is now given the least size, 0.01 cm.
- `cabinet_run room=` takes the room's name as well as its id (`room=Cozinha`), in any case and
  with or without accents. A name was refused as an invalid id.
- `cabinet_run room=` puts the cabinets on the face of the wall the room is on, looked for just
  past the wall, and the notes say which face (`Frentes para Cozinha (r1): no lado de w5 voltado
  para [250, 40]`). The side used to be the one toward the room's centroid, which in an L-shaped
  room lies past its own inner walls: the run came out on the far side, outside the room. A room
  on neither face of the wall is refused.
- A door or window only cuts the walls of its own storey. The same wall drawn again upstairs,
  right over it, was opened too, and could be named the opening's wall in `home` and `measure`.
- A refused `place` no longer uses up ids. The pieces it would have placed took their numbers anyway,
  so after a refused batch of three the next piece came out `f4` instead of `f1`, and an agent
  that had planned its ids found them shifted.
- `layout` checks a solid drawn as an outline by its outline, not by the rectangle around it:
  a shelf drawn turned 10.5° along a sloped wall, or a countertop notched around a wall corner,
  was reported `in_wall` because its bounds reached into the wall. Overlaps between pieces use
  the outline too.
- An `elev` a ceiling piece does not keep is said. A downlight or presence sensor given `elev`
  goes back onto the ceiling, and `update` answered that it "already had the values asked for"
  while it sat 27 cm higher; `place` said nothing. `update` now lists it under `overridden`
  (`elev 240 snapped to the ceiling at 267`) and `place` adds a line per piece after its ids.
- `show_plan` in a client with no viewer (Claude Code, a CLI) no longer answers with the whole plan
  as SVG — 150 to 330 thousand characters the client refused or spilled to disk. The drawing goes
  only to a client that declared the MCP Apps extension (`io.modelcontextprotocol/ui`) when it
  connected; any other gets the one-line summary and the editor link.
- `update` of one end of a wall (`a` or `b`) no longer moves the other one: only the ends given
  are welded onto a wall they nearly touch, and the reply lists each one it moved
  (`welded=w14.b→[615.7,448.7]`), as `create` does. The end left out could be pulled several
  centimetres with nothing saying so, or the change come back as `unchanged`.
- `update` takes `layer` on a label or a line, as `home` shows it there: `electrical`, `plumbing`, or
  empty for the architectural plan. It was refused as a field that does not apply, so a note or a
  line drawn on the wrong project could not be moved to the right one.
- A door hinged on the right (`hinge_right`) has its handle on the left in 3D, away from the
  hinges, as the plan swing shows. The 3D handle always stayed on the right.
- An export to `.glb` or `.obj` holds every storey of the building. It held only the storeys up to
  the one selected in the editor, as the 3D view shows them, so exporting with the ground floor
  selected lost the floors above. `export` now lists the storeys it wrote (`levels=…`); a storey
  hidden on purpose stays out.
- `edit_model action=material` with both an image (`mat:img:…`) and a `color` draws the image
  tinted by the colour. The colour replaced it and the material came out flat. A colour alone
  still repaints the file's own image.
- `update` takes `pts` on a solid drawn from an outline and redraws it there, as `create` would; it
  was refused, and the only way to reshape a slab or a deck was to delete it and draw it again. A
  wall takes `pts` as its two ends (the same as `a` and `b`), and more points are refused saying
  that more walls are drawn with `create`.
- Changing the size or place of a piece inside a group made with `arrange` says to take it out with
  `arrange ungroup` and group again, instead of "belongs to the group", which the group did not
  take either.
- A wall that starts a step aside from an L corner — carrying on from one of its walls a few
  centimetres off the corner's line — no longer has its end pulled into the corner and its
  long faces drawn sloping. It meets the wall it touches as a T.
- `export` to `.png` says the size it wrote (`ok plan.png 2048x1536 px`). A side past 2048 px was
  cut down to 2048 with only `ok <path>` in reply; now the reply also gives the size asked and
  the limits, and `w`/`h` say they are 64..2048.
- `edit_model action=material` refuses an image it cannot read (`mat:img:…` with a wrong path
  or a broken file) and names the full path it looked at, changing nothing. It answered ok and
  the material was drawn without its image.
- Two rooms that follow their walls no longer turn into copies of each other when the wall
  between them is moved away. The first takes the space; the other keeps the outline it had,
  and the reply names it (`rooms_kept`) so it can be deleted or redrawn.
- Adding a storey below the ground floor (`edit_levels add elev=-300`), or changing a storey's
  elevation so another becomes the lowest, no longer moves the ground floor's walls, rooms and
  pieces onto it. What was drawn before any storey existed belongs to "the lowest storey"; it is
  now given that storey by name, in the same step, before the lowest one changes.
- `file save` says what it did to the images and models it bundles: the big JPEGs it stored
  recompressed (name, bytes before and after; the originals are untouched) and the files the
  project refers to that were not found and were left out. It answered a bare `ok` either way.
- A checkpoint set before the project was opened is no longer reverted to. Its place in the undo
  history was saved, but the history was not: after reopening, `history` called it "0 changes
  ago" and `edit_history revert` undid whatever had been done since. `history` now marks it
  `restorable` false and revert refuses it with nothing changed; checkpoints set in this session
  work as before.
- `cabinet_run p.sink=null` (or `cooktop=null`) takes the sink out of the run: the bowl set into
  the old countertop brought it back every time. A sink or cooktop asked for in the call moves
  the bowl or cooktop already there instead of being overruled by it.
- A sink or cooktop asked for where another wall's run turns the corner keeps its place: that run
  is planned again and stops short of it and of the room to open its doors, and stays so when
  either wall is planned again. The sink was pushed along or lost to a blind panel, and a stretch
  the corner panels took whole said "não cabe no vão de -61 cm"; that case is now an error
  naming the cabinets in the way.
- A solid outline or profile enclosing under 1 cm² is refused saying how much it encloses and that
  coordinates are cm (`encloses 0.98 cm²; … metres ×100?`), instead of "needs at least 3 points
  enclosing an area", which did not say the points were read as centimetres.
- `create` names the walls it drew shorter than they are thick (`short=w1`), the mark of a plan
  typed in metres, so the reply says so before the rest of the plan is built on it.
- A `measure` probe through a door or passage no longer reports the wall there: where the
  opening covers the whole height probed (`z`, 0–200 by default) the wall is free floor. Wall
  beside the opening, a lintel or a window sill inside that height still count.
- The first storey `edit_levels add` makes in an empty project is the only one. An empty
  "Térreo" was created beside it, at the same elevation, and the reply named only the storey
  asked for. Over a plan already drawn, the ground storey made for it is still added, and `edit_levels` now names both
  (`ids=lv1,lv2`, and a line saying which holds what was drawn).
- `edit_levels` says that storeys belong to each plan version: with several versions, its reply
  names the one it changed (`storeys are per version: this changed v=4 only`), and its
  description says a change applies to the active one.
- A countertop changed with `joinery` (shorter, with its cutouts moved) keeps the real sink and
  cooktop set into it in their cutouts: they kept their distance from the countertop's middle,
  so cutting 76 cm off one end moved the cooktop 38 cm off its hole. One whose cutout is gone
  stays where it stood, and the reply says so.
- Brick given a colour (`brick #fbfaf7 25x7`) is drawn as painted brick in that colour: even
  bricks with shadowed joints. Its strong light-and-dark tones and light mortar turned a white
  brick wall into grey mottle. Brick with no colour is unchanged.
- A joinery build resized with `update` (`d=60 h=45`) keeps that size when it is changed again
  with `joinery` (`p.color`): it was rebuilt at the size its parameters remembered. The reply
  notes the sizes it took; a size given in the change still wins.
- An unknown wall `type` is refused with the list of wall types there are, instead of a pointer
  to the `materials` tool.
- `glass` (on `place` and `update`) lists its four kinds in the schema, so a client can offer them
  and check them before the call.
- The schemas say what agents got wrong: `v` is the index of a plan version in `variants`
  (0-based), not a revision; `render_3d` `yaw` and `pitch` do not go with `piece`; and `fixed`
  marks a piece built-in, not locked in place.
- A cabinet's, run's or sofa's `color` takes `#rrggbb` as well as `[r,g,b]`, as `handle_color`
  already did: `p.color="#d8d2c6"` was refused as an invalid type.
- A cooktop embedded in a countertop counts as the kitchen's cooking zone whatever it is named:
  the review read embedded items by their Portuguese names, so a `Варочная панель` set into the
  stone left the kitchen "missing" its cooktop. Embedded items are now read by what their host
  says they are, however deep in the group, and catalog `cooktop` and `sink-bowl` pieces count
  as a cooktop and a sink.
- `update` resizing a group and renaming one of its parts in the same call resizes the parts
  too, in either order: they kept their old size and place, and with the rename first the resize
  also undid the rename.
- A group made with `arrange` faces the way its largest piece does, with its box measured that
  way: a wardrobe facing -x grouped with its nightstands made a group at 0° that was reported as
  `turned` against what it holds, and a box wider than the pieces.
- On macOS the editor no longer closes while a file dialog is open (open, save, export, import,
  photo, video, a finish image). The dialog used to wait for its answer inside the frame that
  opened it, and an event that arrived meanwhile closed the program. File dialogs now open as a
  sheet on the window (a dialog of their own elsewhere), and the window keeps drawing until the
  answer comes.
- An editor that closes at launch because of its graphics driver no longer does so on every
  launch: if the last launch closed before it drew, the next one tries the next graphics backend
  of the system (on Windows Direct3D 12, then Vulkan, then OpenGL; on Linux OpenGL) and keeps the
  one that works until the next version. `WGPU_BACKEND` still chooses by hand. A GPU lost while
  the editor is open is now named as such in the crash report.

### Security

- Dependencies updated past the open advisories: `zip` 9 (archive extraction through symlinks,
  SUID bits), `rustls-webpki`, `webpki-roots` and `webpki-root-certs` (distrusted emSign
  intermediates) and `ctutils` (non-constant-time comparisons). Also `glam` 0.34, `rmcp` 3.5,
  `tokio` 1.53.3, `serde_json`, `schemars`, `sha2`, `uuid` and `winresource`.

## [3.1.0] - 2026-10-01

### Added

- Radiators in the catalog: `radiator-60`, `radiator-100` and `radiator-120` (panel, 10 cm deep,
  50 cm tall) and `convector-100`, hung 15 cm off the floor and found by `radiador`,
  `radiator`, `aquecedor`, `heater`, `convector` or `hvac`. A radiator under a window had to be
  drawn as a generic solid, with no place in counts.
- `create exact=true` keeps every wall point where it was given. Otherwise wall ends that nearly
  touch a wall are still pulled onto it, and the reply now lists each one it moved
  (`welded=w14.a→[615.7,448.7]`). Walls traced from measured pixels came back with ends moved
  and nothing saying so.

### Fixed

- The editor no longer closes at launch on older Intel Macs (macOS 12): wgpu's validation of
  indirect draws builds a compute pipeline the Metal compiler there fails on, which lost the
  device. Nothing draws indirectly, so the check is off (`WGPU_VALIDATION_INDIRECT_CALL=1` turns
  it back on).
- A wall meeting a stepped junction — the end of a wide pier beside its axis — no longer shares
  that junction and gets a sloping long face in the plan.
- `catalog cat=<unknown>` answers with the categories there are instead of an empty list.
- A paste with nothing readable on the clipboard is no longer reported as an error.

## [3.0.0] - 2026-09-29

### Added

- A plugin as heavy as a modeler declares `limits` in its `plugin.json` — `heavy`, `threads`,
  `memory_mb`, `nice`, `timeout_s` — and they are applied to its process: on Linux it is pinned
  to that many cores, its memory is capped and its priority lowered, and it gets
  `NEWERA_THREADS` for the program's own thread flag (Blender's `--threads`). A heavy plugin
  runs alone, sharing one slot with photo renders, so the machine's budget is not multiplied by
  every caller. `run_plugin` answers the peak memory and processor time it used (read from
  `/proc`; not measured elsewhere), the limits applied and those the system could not apply.
  An external modeler ran with whatever affinity and priority the agent remembered to set.

- A model library, apart from projects: `edit_library action=publish` keeps a piece's model —
  with the files it names, its product photos and measurements, unit, size, brand, page, notes on
  what changed and how faithful it is — as the next version of an entry, never changed after;
  `library` lists entries and versions. `place model=library:<name>` (or `@<v>`) puts one in any
  project with its photos and measurements, at the size and in the unit it was published with,
  and `edit_model replace file=library:<name>@<v>` goes back to an older one under the same id.
  The library is on the desktop only (`NEWERA_LIBRARY`, or the config folder). Reusing a chair
  meant finding its files again, and its photos and checks stayed in the study's folder.

- `render_photo` answers for the machine it runs on: `estimate=true` gives the thread budget
  (half the cores, at most four, one render at a time) and the work — pixels, rays, triangles,
  images, memory estimated from their sizes — without rendering, and says that time,
  temperature and power are not measured; `threads` lowers the threads it may use, `max_s`
  stops it past a deadline with nothing changed, and `cancel=true` stops the one running.
  `sessions` shows the render running (`tool`, seconds, phase, progress, threads). An agent
  could only pick a quality and a size, and a render already running was a refusal to wait on.

- A piece keeps photos and measurements of the product it reproduces, with where they came
  from. `edit_model` `action=reference` adds a photo (`file`, `view`, `part`, `source`, `note`;
  `remove`, `clear`), which travels with the project; `render_3d piece=… ref=i` puts it beside
  the model, drawn from the photo's side and part. `action=measure` keeps a product size
  (`dimension`: `width`, `depth`, `height` or `<part>.height`; `cm`, `source`, `confirmed`), and
  `model id=…` lists each against what is drawn, warning when they differ by more than a
  centimeter — a seat at 49.5 cm checked apart from the chair's overall height. Photos had to
  be downloaded and compared by hand, and a size's source and meaning were not kept.

- `render_3d` `piece=<id>` draws that piece alone — no walls, rooms or other pieces — seen from
  its own front, back, left, right or top (`aerial` is three-quarter), with `zoom`; the same call
  frames the same view wherever the piece stands. `part=<name>` frames one part of its model,
  such as the tip of an arm. Checking a chair against its photos meant a stored camera by
  coordinates, with the armchair next to it in the edge of the picture.

- A piece with a heavy imported model can have a lighter file for afar: `edit_model`
  `action=lod` (`file`, `beyond` cm from the camera, default 400) draws it in the 3D view, the
  previews and photos when the camera is farther than that, in the same box, its materials and
  parts answering by the same names; `detail=always` holds the detailed file for a close review,
  and exports and top views always use it. The lighter file travels with the project, and one
  that cannot be read leaves the piece drawn in full. `model` says where the triangles go — `cost {tris,
  heaviest [[part, tris, %]], copies, in_project}` — and warns when a model over 50,000
  triangles has no lighter file. The Ares chair's weave was 69 % of its 89,718 triangles, and
  nothing said so.

- A texture's alpha can cut holes: a glTF material with `alphaMode: MASK` (and its
  `alphaCutoff`), or an MTL whose `map_d` is its color image, has holes wherever the image's
  alpha is under the cutoff — in the 3D view, the previews and top views, and photos, where
  rays and sunlight pass through them — and is seen from both sides when the file says so (an
  MTL mask always is). Exports keep it: GLB as `MASK`, OBJ as `map_d`. A cane weave had to be
  modeled strand by strand: 62,256 of the Ares chair's 89,718 triangles.

- `edit_model` `action=part` hides, moves (`offset`, cm: x across, y to the front, z up) or
  resizes (`scale`, about its center) one named part of an imported model — the back cushion
  alone, the arms where they were — and `clear` puts it back. Every view draws it that way, and
  `model id=…` lists the parts as drawn (a hidden one marked `hidden`, and left out of
  `check=clashes`), so seat and arm heights read on their own. Moving one cushion meant a new
  file.

- `edit_model` `action=material` changes one material of an imported model by the name `model`
  lists — `color`, `mat` (an image over it), `repeat` (its texture scale: 2 draws the image at
  half size, the piece unchanged), `clear` — in one undo step. Per-material overrides existed
  (a Sweet Home 3D import brings them) but no tool could set one, and a weave drawn too large
  could only be fixed in the file.

- `place model=…` takes `unit` (`m`, `cm`, `mm`, `in`) for the file's numbers, and the piece
  keeps it for `model` and `edit_model`. The unit was only guessed from the size — under 20 units
  meters, over 2000 millimeters — so a 5 cm knob drawn in centimeters came in 5 m wide, and an
  800 mm chair 8 m tall. When the guess makes a piece smaller than 10 cm or larger than 4 m, the
  reply says so. A glTF whose numbers are not meters, as the standard says they are, is warned
  about too.
- A model placed with one or two of `w`/`d`/`h` scales evenly, keeping its proportions; sizes that
  would stretch it unevenly are refused unless `stretch=true`, and then the reply gives the
  scale on each axis. Asking a chair for its height stretched it.

- `model` `check=clashes` finds the parts of an imported model that pass through one another —
  a cushion through a rail — with the triangle pairs, a point where they cross and how deep, in
  cm of the piece; surfaces that only touch (a cushion resting on the frame) are not a clash,
  and `tol` sets how far one has to pass the other. It also counts faces with no area, and says
  what it did not look at (a part wholly inside another, clearances, flipped normals, open
  meshes). The check had to be a script in Blender.

- An imported model keeps its named parts — OBJ objects and groups, glTF nodes — and `model`
  lists them: `[name, triangles, min, max, materials]`, in cm of the piece (x across, y to the
  front, z up), with `part=<words>` to keep only some (`braco`). A repeated name gets `#2`. The
  names come from the file, so they are the same after saving and reopening. The chair was one
  lump: telling the left arm from the right, or the seat cushion from the back, meant opening
  the file in another program.

- `edit_model` `action=replace` puts a new version of a model file in placed pieces in one undo
  step, keeping each piece's id, position, angle, storey, size (`size=natural` takes the file's),
  finish, material overrides, names and links; `every=true` does it for every piece on the same
  file. The new file is read first and nothing changes when it fails; `rev` refuses the change
  when the plan moved on since the revision it was prepared on. The reply names the version
  loaded (bytes and a hash), the overrides that no longer match a material the file uses, a kept
  size that stretches the new file unevenly and what the file does not draw. A new version was
  `place` then `delete`: two pieces in one spot between the calls, a new id, and whatever the
  person had set on the old one lost.

- `model` reads what an imported OBJ/glTF/GLB became: the unit taken for its numbers (and the
  numbers themselves), its natural size, triangles, each material with its color, image and
  triangle count, and warnings for what is not drawn as the file says — a material library or
  texture not found, normal, roughness and opacity maps, an alpha mask, emission, an extension,
  animations. Given a placed piece (`id`), it also says the scale per axis and warns when the
  piece is stretched unevenly. `place model=…` adds a line per model with such warnings, and
  importing from the window says them in the status bar: "it loaded" read as "it looks right",
  and finding out why a chair came in gray meant reading the importer.

- Win armchair and Ares cane chair in the living and dining catalogs, with
  product dimensions, orientation, model metadata and bundled textured models. Models
  and their textures travel with saved projects; placement uses the existing catalog
  and MCP tools. Product proportions are preserved when resizing.

### Changed

- **Breaking for clients that validate output schemas:** `render_photo`'s output schema no
  longer requires `mimeType`. `estimate=true` and `cancel=true` answer in text (JSON), not
  with an image, so a client that required `mimeType` on every reply must accept both.

### Fixed

- A glTF/GLB model keeps its textures. The importer read positions, normals and the base color
  factor only, so a chair whose wood and fabric were images inside its GLB came in as flat gray
  and white, and the way out was re-exporting it as OBJ. It now keeps each material (its name, the
  base color image and, for `BLEND`, its alpha) and the texture coordinates of the set the image
  uses; images inside the GLB or in data URIs are read like any texture file, and one next to a
  `.gltf` is found by its URI, `%20` and all — a URI that would leave the model's folder
  (`..`, an absolute path, a scheme) is refused, for buffers as well. The materials keep their names, as an OBJ's do,
  so a per-material override (from a Sweet Home 3D import) finds them.

## [2.0.0] - 2026-09-28

### Added

- `ergonomics`, `electrical`, `plumbing` and `lighting` take `level`, the storey to review, as
  `layout` and `home` already did. They reviewed only the storey shown in the editor, so an agent
  had to switch the person's view to review another floor, and a review of the upper floor, left
  shown, quietly became the review of the house. The storey shown is left as it was.
- `render_3d` takes `level`: a storey draws it and the ones below, `all` draws every one. It drew
  what the editor showed — the selected storey and those below — so the same camera rendered the
  stair's landing without the floor above it when the ground floor happened to be selected, and
  checking the connection meant switching the person's view.
- A flight of stairs built by hand is a stair: `update(role="stair")` on the group or the solid
  that draws it opens its hole in the floor of the storey it climbs to, as the catalog's flight
  does. Before, only the catalog's did, so a staircase of drawn treads and landings meant cutting
  the upper room's outline around it by hand.
- A `rules` tool answers what a review checks against — the NBR 5410, 16264, 5626 and 8160
  tables behind `electrical` and `plumbing`, what each `check_layout` problem means, how
  `ergonomics` scores — and the same texts are resources, `newera://rules/<topic>`.
- The MCP tool surface keeps its own rules as it grows, as tests (`docs/MCP-TOOLS.md` lists
  them): a byte budget per tool that only goes down, names that say whether a tool reads or
  writes, descriptions that neither order the model around nor name a tool that is not there,
  choices typed as enums, unknown arguments refused, and a BM25 tool search over the surface
  that must keep finding the right tool for 82 requests agents make.
- A project says which country and state it is in — `set_home(country=…, region=…)`, beside
  the city it already had — and that decides which codes judge it. A project in the United
  States is reviewed against the IRC 2024 (room areas and sides, ceiling heights, daylight, the
  clearances in front of fixtures, hallways), the IRC and NEC kitchen outlets and ICC A117.1
  for a wheelchair; what those codes leave unsaid is still measured by the Brazilian standards,
  as a reference.
- Rooms named in English — "Bedroom", "Kitchen", "Powder room" — are recognised by every check,
  electrical, plumbing, lighting and Wi-Fi included; before, nothing recognised them.
- Review findings can be said in English, and the window shows them in English when it speaks
  anything but Portuguese. What they are accepted by does not change.
- Installing and uninstalling are counted, per system: the three installers post one event to
  `3dneweraai.com/ping` saying which operating system and architecture, whether the app came
  from a published release or was compiled on the machine, and whether this was the first
  install. They draw the installation id the app then uses, so an install and the first run are
  one installation in the count and not two.
- Crash reports from builds that carry no DSN — what compiling the app yourself gives, unless
  `NEWERA_SENTRY_DSN` is set for the build — now reach Sentry through `3dneweraai.com/ping/sentry`, which signs the envelope with the DSN the
  site holds. Those crashes were seen by nobody before.

### Changed

- **Breaking: the MCP tools have new names, and some are one tool now** (59 → 53, the server
  instructions list them all by what they are for). A read is a noun and the tool that changes it is
  `edit_<noun>`: `get_home` → `home`, `set_home` → `edit_home`, `check_layout` → `layout`,
  `trace_background` → `background`, `fill_lighting` → `edit_lighting`, `checkpoints` → `history`.
  `new_home`, `open_home` and `save_home` are `file` with `action`; `undo`, `redo` and `checkpoint` are
  `edit_history`; `split_wall` and `merge_walls` are `edit_walls`; `set_background` and `trace_walls`
  are `edit_background`; `export_plan` and `export_cut_list` are `export` (`what=cut_list` for the
  cut list). The skills, the hosted service and the browser tab speak the new names; an agent that
  calls an old one is told the tool does not exist.
- The tool list an agent reads on every conversation is a quarter smaller (149 KB to 113 KB): the
  reviews' descriptions say what they answer and point to `rules` for the tables, output schemas go
  out without their field descriptions, and defaults that only restate absence are left out. The
  descriptions use the words people search with, so a tool search finds the right tool for all 86
  requests in the test set, and the first three hold it for 96.5 % of them (90 % before).
- A tool that refuses a call answers inside the result (`isError`) on the desktop and `newera mcp`,
  as the hosted service and the browser tab already did, so the agent reads why and tries again
  instead of a client showing a protocol error to nobody.
- Every tool refuses an argument it does not have, by name and with what it takes instead —
  before, forty of them dropped it in silence, so a misspelled `dry` wrote. An argument given with
  an action it does not belong to (`dx` with `rotate`) is refused the same way. Every choice
  (`action`, `kind`, `view`, `quality`…) lists its values in the schema, and `dry`, `fixed` and
  `facing` say what they take when given something else.
- How much a standard obliges depends on where the project is: an ABNT standard is an error in
  Brazil and a warning in Miami, and the IRC the other way round. Electrical, plumbing and
  railing findings follow the same rule, so NBR 5410 no longer accuses a project abroad.
- The lighting reply cites the table each reference value comes from — NBR 5413 or
  NBR ISO/CIE 8995-1 — instead of both every time.
- The usage count no longer needs a secret inside the binary: it goes to the site's own
  endpoint, which holds the Google Analytics secret and forwards it. Releases were being built
  without that secret, so nothing was counted at all; and a build from source counted nothing by
  design. Both now count the same.
- The count says the country Cloudflare already knows from the connection, and the visitor's
  address is not passed on to Google.
- A debug build and CI count nothing and report nothing, which is what the missing secret used
  to accomplish by accident.

### Fixed

- A door, a point or a cabinet put on a wall now stands on that wall's storey. Before, `place`,
  `joinery` and `cabinet_run` put it on the storey shown: a door given a ground-floor wall while
  the upper floor was selected was made upstairs, loose and opening nothing, and an outlet on
  that wall was refused as having no wall to hold it. Fixed points are also judged against the
  walls and ceilings of their own storey.
- `layout(level="all")` no longer lists every piece that stands over another on the storey
  below. Pieces of two storeys were an overlap wherever their plans crossed — a sofa downstairs
  and a bed upstairs, a roof over the furniture — which buried a real problem under hundreds of
  `cross_level` rows with no height shared. They are now one only where they share height in the
  building (storeys drawn at the same elevation, a piece reaching into the storey above), and
  the extent's z is that shared height.
- `ergonomics` judges a bathroom as a bathroom. A bowl set into a vanity — a "cuba" — was read as
  a kitchen sink wherever it stood, so a bathroom with a countertop and a bowl, or a double vanity,
  was reviewed as a kitchen (the five work zones, 366 cm of counter) and still said to have no
  basin. A bowl in a bathroom is now a basin; a bathroom or a laundry is never reviewed as a
  kitchen; and the cabinet under a vanity asks the basin's 40 cm in front, not a kitchen's 85.
- `ergonomics` counts the whole house for what it says of the house. Beds, bedrooms, bathrooms,
  seats at the table and wardrobes were counted on the storey shown only, so reviewing the upper
  floor of a house said there was no seat at the table and no bathroom — and advised adding them.
  `capacity` and the occupancy findings now add up every storey being designed.
- A glass sliding door lights and airs its room in `ergonomics`, as the French window already
  did. The catalog's `door-sliding` is drawn with glass leaves, but only windows were counted, so
  a living room opening onto its balcony through a 220 × 220 cm glass door was said to have no
  window.
- `edit_lighting` keeps every fixture it places whole inside the room. It kept only the centers
  inside, so in an L-shaped room a column of the grid could fall a few centimeters from the inner
  corner's edge and the fixtures there crossed into the next room (`layout` then called them
  `outside_rooms`). A fixture now stands at least half its diagonal off every edge.
- A guard in front of a window no longer blocks it in `layout`. A bar railing, a glass
  balustrade or a balcony closure was measured by its box, as if it were a solid panel, so a
  landing guard across a stair window came out as `blocks_window`. Guards are now left out, like
  plants and glass already were.
- `home` says a piece's finish, as `mat`, in the words `place` and `update` take it. It said a
  wall's and a floor's, never a piece's, so a finish set on a cabinet front could not be read back
  to check it had taken — and a dry run of a new finish answered that nothing changed.
- A finish that is only a color — `mat:"#c8b89a"` on `place`, `update` or a solid — paints the
  piece. It was stored as a finish the 3D does not draw (a piece draws a pattern or an image), so
  the call was accepted and nothing changed on screen; it now sets the piece's color, unless a
  `color` is given in the same call.
- An item set into joinery — a sink bowl, a cooktop, an oven — takes its own `color`, `mat` and
  `opacity` in `update`. It was refused like a board the host builds, so repainting a bathroom
  meant leaving the bowl as it was or changing the countertop around it. The item is carried whole
  when its host is rebuilt, and its finish with it; the host's own parts still refuse.
- `layout` says a group outside every room once. Each of its parts was a row, so a facade panel
  of 42 slats was 43 `outside_rooms` rows next to the four parapets; a group partly outside still
  names the parts that are.
- `plumbing` reads a bowl by the room it is in. Every "cuba" was a basin — a 40 mm branch, one
  fixture unit — so the kitchen sink was asked for a basin's drain and no grease trap, and the
  laundry's bowl the same. A bowl in a kitchen (or a gourmet area) is now a kitchen sink, 50 mm and
  a grease trap as NBR 8160 asks, and in a laundry a laundry sink; in a bathroom, and anywhere
  else, it is still a basin.
- `plumbing` names each fixture with its own article: "a pia de cozinha", "a máquina de lavar",
  "a banheira" — it said "o" for all of them — and a missing cold-water point reads "não tem de onde
  receber água", which agrees with either. Keys, and acceptances given before, are unchanged.
- `ergonomics` lets a washer, a dryer or a dishwasher stand under a countertop with a sink in it.
  A countertop was measured by its box, which reaches the floor, and one with a sink cutout no
  longer counted as the cabinetry things go under — so the laundry's washer and dryer, set under
  its counter, were each an error, "ocupa o mesmo lugar", with nowhere to move. A countertop is now
  its slab; a piece that reaches into the slab still clashes with it.
- A tall cabinet in a kitchen is its storage in `ergonomics`' five work zones. A joinery cabinet
  taller than 1.5 m is read as a wardrobe (for its doors' clearance), which the zones did not count,
  so a kitchen with a 235 cm pantry tower was told storage was missing.
- `embed` cuts the wooden top of the cabinet under the stone too. A bowl or a cooktop set into a
  countertop got its hole in the stone, but the joinery cabinet below kept a whole top, so the
  render showed the board through the bowl and no check noticed; each one had to be cut by hand
  with `top_cutout`. The cabinets it cut come back as `cut_below`, in the same undoable step; one
  whose top already has a cutout elsewhere is left alone and named in the notes. `cabinet_run`
  cuts the tops of its sink and cooktop cabinets the same way.
- `place` with several items says every one it refuses, and that nothing was placed. It stopped
  at the first refusal, so a batch of lights with two misplaced ones took two calls to learn about
  both, and the refusal did not say whether the others had gone in. A single item is refused in the
  same words as before.
- `home` gives `elev` for every piece off the floor. It left it out when it matched the catalog's
  default, which `catalog` does not say, so a LED strip placed without `elev` — 140 cm up, for under
  a cabinet — read as if it were anywhere, and a profile meant for the ceiling was found hanging in
  the render instead.
- The catalog's straight flight keeps its steps a step high when it is resized. It always had 16,
  so one resized to a 1.31 m landing, 2.24 m deep, climbed in 8 cm risers on 14 cm treads, in 3D
  and in plan. Its risers now follow its height, the fewest that keep each at most 18 cm (NBR 9050
  asks 16 to 18): 8 of 16.4 cm there, and still 16 at the catalog's 2.8 m.
- The ceiling under a stair's void is open. A stair climbing to the storey above opened its hole in
  that storey's floor, but the ceiling of the room it climbs from stayed closed over it — in 3D, to
  the lighting and to the ceiling checks — so seeing the stair from either floor meant turning that
  room's whole ceiling off. The same void is now cut from it.
- `annotations(stale=true)` reads a size written in meters as meters when it says so. "Piscina —
  7,40 × 3,00 m" was taken for 7.4 × 3 cm — a group was read as meters only when every number had
  a fraction — and reported stale against its own 740 × 300 cm. A unit written after a group (or
  onto its last number) now says how to read it: `m` meters, `cm` centimeters, `mm` millimeters
  ("600 × 600 mm" was read as 6 m).
- `ergonomics` measures a TV from the seats that can watch it. It took the nearest sofa or
  armchair whichever way it faced, so a reading chair beside the TV, its back to it, was "121 cm
  from a 56-inch TV" while the sofa facing it sat at a comfortable 3.1 m. Seats turned away from the
  screen are left out.
- An empty total is 0, not -0.0: `electrical(action=circuits)`'s `total_va` with no circuits yet, and
  `lighting`'s `lm`, `W` and W/m² for a storey or a room with no light — the sum of nothing in
  floating point is negative zero, and it went out as such.
- The hosted service listed tools it then refused — the video, the scanned plan — and a browser tab
  listed feedback and the plugins, which need a desktop. Each place now lists only what it runs, says
  so in one place for all three (`newera_mcp::surface`), and gives server instructions that name every
  tool it offers by what it is for; a tab sends its own through the relay.
- Tool descriptions that told agents something untrue: `electrical` promised that `wifi` writes
  the access points' standard (only `edit_electrical` does), `render_3d` listed two of its seven
  views, `open_home` called its required `path` optional, and `check_layout` left `backwards`
  out of the problems it lists. A doc comment meant for a function sat on the dry-run type and
  was sent, 583 bytes of it, in three tools' schemas.
- São Paulo's minimum room areas and sides no longer judge a project that says it is in another
  city, and the São Paulo state kitchen minimum no longer judges another state.
- A building code picked under "Código de obras" in the desktop is kept when the project is
  saved; it used to be forgotten.

## [1.10.0] - 2026-09-26

### Changed

- The paid plan is supported through Buy Me a Coffee instead of Paddle, whose
  verification a seller in Brazil could not get through. The account page sends people to
  the Buy Me a Coffee membership page and the webhook (`/billing/buymeacoffee`) follows
  the `membership.*` and `recurring_donation.*` events. A cancelled membership keeps the
  plan until the end of the period already paid for, as before.
- What ties a payment to an account is the address it was paid with, not an id carried
  through the checkout: Buy Me a Coffee's page takes nothing of ours along. The support
  page shows the account's address and asks for it, and an address with no account yet
  gets the one a sign-in link would have made.
- Closing an account no longer stops the charges: Buy Me a Coffee gives a seller no way
  to cancel a membership. The account page says so before and after closing, and the
  plan stops here either way. The warning counts the subscriptions left from Paddle too,
  which used to be cancelled through an API this no longer holds a key for.
- A support button, in Buy Me a Coffee's yellow, at the left of the bar on the site, at
  the right end of the editor's tool bar — desktop and browser — and on the account
  pages. It follows the language around it; the membership is charged in US dollars
  wherever it is pressed. Too narrow for words, the editor's becomes one more button the
  size of the tools beside it, with the cup alone, and the tools scroll underneath rather
  than pushing it off the edge.
- The pricing, terms, refund and privacy pages show one language instead of stacking two.
  They pick it the way the home page does — `?lang=`, then what was last chosen, then the
  browser — and carry a PT/EN switch. Both texts stay in the page for a reader without
  JavaScript and for a crawler, and the tab's title follows the language too.
- Those four pages name the project's maintainer rather than a company, and no longer
  claim a merchant of record: Buy Me a Coffee is a platform, not a reseller, so the
  seller is whoever maintains the project and the tax is theirs. Prices are stated in US
  dollars, the currency actually charged — Paddle's localised checkout is gone.
- Documentation for contributors, in English: a rewritten README, a docs index,
  `docs/STANDARDS.md` (was `NORMAS.md`), `docs/DISTRIBUTION.md`, and `SECURITY.md` and
  `CODE_OF_CONDUCT.md` at the root. `make` lists its targets in English.
- The example plugin is `examples/plugins/room-areas` (was `plugins/quadro-areas`), in
  English. Showcase images have English file names.
- The review bot reads its rules from `.coderabbit.yaml` in the repository instead of a
  dashboard nobody outside the org can see: clippy treated as a build failure, the
  `newera-mcp` tool shapes treated as a public contract, and outside contributors
  allowed to talk to the reviewer on their own pull request.
- A prose-only change no longer runs the whole CI. Editing a README, `docs/` or an issue
  template used to spend eighteen minutes across seven jobs to prove that Markdown still
  compiles. Markdown runs a Docs workflow instead — markdownlint against rules the
  repository carries in `.markdownlint-cli2.jsonc`, spelling, and the relative links
  between pages, so a renamed page cannot quietly orphan the link to it. `make docs-lint`
  runs the same three locally. A licence file runs nothing at all, having nothing any of
  the three can check. The plugin's own Markdown is exempt from the exemption: the Format
  job validates every `SKILL.md`.
- `main` is protected: a pull request with its checks green is the only way in, force
  pushes and deletion are refused, and history stays linear. The path filtering that
  skipped prose moved out of the workflow triggers and into a `Changed paths` job, because
  a required check that never reports blocks a pull request forever while a skipped one
  counts as a pass. `scripts/changed-kind.sh` decides, and proves itself against a table
  of cases on every run.
  A single `CI` and `Docs` check speaks for the jobs behind it, because a matrix job that
  is skipped reports once under its unexpanded name and the per-OS names would never
  arrive on a prose-only run.

### Removed

- Internal planning notes and launch material from `docs/`, personal helper scripts,
  and the `packaging/` snapshots of 1.7.0; releases generate the winget and Homebrew
  files themselves.

### Fixed

- The ground no longer shows through a doorway. A floor stopped at the inner face of the
  walls around it, so the band under a wall belonged to no room and the grass showed
  through wherever a door or a passage cut the wall away. Floors now run under the walls
  they touch: to the middle of a wall with another room behind it, where the two floors
  meet as they do at a real threshold, and through to the far face of a wall with nothing
  behind it.
- A long name no longer pins the side panel open. A plan note a paragraph long
  ("AÉREOS: 80 geladeira + 106 cozinha…") made the panel as wide as itself and the drag
  handle could not bring it back. Names are cut short with an ellipsis and shown in full
  on hover, and a click on a name now selects its row instead of selecting the text.

## [1.9.2] - 2026-09-24

### Changed

- mcp.3dneweraai.com, opened in a browser, goes to 3dneweraai.com instead of a blank
  "not found".

## [1.9.1] - 2026-09-24

### Changed

- The paid plan is sold through Paddle instead of Polar, which cannot pay out to a seller
  in Brazil. The account page opens Paddle's checkout (`/billing/checkout`), shows the
  price in the buyer's currency, and "Manage subscription" opens Paddle's customer portal.
  Closing an account stops the charges at the end of the paid period.
- The site has a pricing page and a refund policy (full refund within 14 days), and the
  terms name the seller and Paddle as the merchant of record.

## [1.9.0] - 2026-09-24

### Added

- Every tool declares an output schema, and every answer carries `structuredContent`
  laid out from what it already says: a JSON object as it is, a list under `rows`, an
  `ok rev=N ids=…` line split into its fields, an image as its type. The text the tools
  answer is unchanged.
- Every argument of every tool says what it is, nested shapes included.
- The MCP handshake names the server's icon, description and website.
- The plugin points at the hosted server (mcp.3dneweraai.com), so it works with nothing
  installed; `3d-new-era-ai-desktop` keeps the local app and its live window.
- A public page at 3dneweraai.com/connector/ says how to add the hosted server to Claude
  and ChatGPT.

### Changed

- Every hosted tool declares the OAuth scheme it needs, and sign-in accepts the
  `offline_access` scope ChatGPT asks for to stay signed in.

### Fixed

- With the walls down or cut away in the 3D view, what is held up from the floor stays
  however close to a wall it stands: a worktop on its cabinets, a desk top on its frame,
  a vase on the worktop and a cabinet's own top board used to vanish with the wall. Only
  what hangs on a lowered wall goes now, and whatever rests on it.
- The site's one-click buttons and legal links are big enough to tap.

## [1.8.0] - 2026-09-24

### Added

- The plan shows up inside the chat: `show_plan` opens an interactive viewer (pan, zoom,
  a 3D view on a click, "open in the editor") in clients that speak MCP Apps — Claude,
  ChatGPT, VS Code and others.
- Every tool says whether it only reads or changes the plan, with a title, so an AI
  client runs reads without asking and asks before each change. Reads and changes are
  separate tools now: `cameras`/`edit_cameras`, `levels`/`edit_levels`, `video`/
  `edit_video`, `variants`/`edit_variants`, `checkpoints`/`checkpoint`, `plugins`/
  `run_plugin`, `disciplines`/`edit_disciplines`, `electrical`/`edit_electrical`,
  `plumbing`/`edit_plumbing`, `lighting`/`fill_lighting`, `annotations`/
  `edit_annotations`, `trace_background`/`trace_walls`, `cut_list`/`export_cut_list`;
  findings of every review are accepted with `accept`, and who lives there is kept with
  `set_home(people=…)`.
- `newera mcp` (stdio) edits the plan in the open window when the editor is running;
  `--standalone` keeps a project of its own.
- One-click installs: `newera-mcp.mcpb` for Claude Desktop in every release, a plugin
  with skills for Claude Code, Codex, Cursor and Gemini CLI, and buttons for Cursor, VS
  Code and LM Studio.
- Accounts at mcp.3dneweraai.com: sign in with an emailed link, and Claude, ChatGPT and
  other AIs reach your editor tab at one fixed address, with no secret to paste. With no
  tab open they work on projects kept in your account, and exports come back as links.
- Privacy policy and terms of use on the site.

### Changed

- The editor's AI link comes back by itself, at the same address, when the service
  restarts.
- A project opened by link (`?project=`) opens even when the tab had work of its own to
  restore.

### Fixed

- The AI link no longer stays on "connecting" forever when the relay refuses a tab.
- `show_plan` said "-0 m²" for an empty plan.

## [1.7.0] - 2026-09-24

### Added

- Axes in the top right corner of the 3D view, as in Blender. Click a ball to look at the
  house from the front, back, left, right or top; drag the disc to orbit. The top view
  reads like the plan, x right and y down.
- Walls in the 3D view can come down to show the rooms: up, cutaway (the walls between
  the camera and a room drop to 40 cm and follow the camera as it turns) or down.
  Ceilings, roofs and the doors, windows and pieces hanging on lowered walls hide with
  them. Three buttons in the toolbar and the View menu switch it.
- `render_3d` takes `walls=cutaway|down` to draw the same way, so an agent can see the
  furniture of a room from the side.

### Fixed

- The selected tool and the AI button in the toolbar no longer change size when the
  pointer goes over them, which nudged the buttons beside them.

## [1.6.0] - 2026-09-21

### Added

- Every catalog item is marked with its front and back, and with whether its back belongs
  on a wall (bed, sofa, wardrobe, toilet, TV), stands free (armchair, chair) or has no
  front at all. The catalog tool lists it, and a test keeps new items from going unmarked.
- `place` takes `facing` (`+x`, `-x`, `+y`, `-y`, a point or an element id) instead of an
  angle, and its reply says where each front ended up. A piece whose back belongs on a
  wall, placed beside one with no angle, turns its back to that wall.
- `check_layout` reports `backwards`: a sofa, bed or toilet facing the wall its back
  belongs on, with the angle that turns it around.
- `catalog_faces` example renders every catalog item from the front and from the back.

### Fixed

- Pendants, spots, LED panels and ceiling points stay on the ceiling the 3D draws — flat,
  sloping or under a roof — through every edit, storey height change and project load.
  A pendant keeps its shade height and its cord takes the difference.
- Recessed spots set into the slab are no longer reported as above the ceiling.
- The editor's WebGL notice names the real cause on Chrome for Linux, where WebGPU ships
  turned off, and says how to turn it on.

### Changed

- rmcp 3.4.0 (`ServerConfig`), clap 4.6.7, and current GitHub Pages and Wrangler actions.

## [1.5.0] - 2026-09-20

### Added

- Browser image and video rendering in disposable workers, with progress, cancellation,
  time estimates and resource limits based on device capabilities. The page and MCP
  connection remain responsive while rendering; only one heavy render runs at a time.
- The AI robot control has its own toolbar group after Frame, preserving connection
  colors and activity feedback.
- Layout feedback for blocked door approaches, tall furniture and hoods in front of
  windows, and unused kitchen corners. Compact countertop appliances are distinguished
  from tall obstructions; unused corners are reported without automatic rearrangement.
- Explicit room use independent of room names, scoped architectural review, and MCP
  controls for flat or wall-following ceilings with preview and undo support.
- Confirmed browser recovery snapshots, downloadable project backups through MCP, and
  structured diagnostics that remain available after a fatal WASM error.
- Rated LED emitters for coves and shadow gaps, cabinet countertop cutouts, and a reviewed
  project with presentation views in `docs/projects/upper-west-side-revisao.newera`.

### Fixed

- MCP plan rendering no longer uses native filesystem paths in the browser. Scene buffer
  uploads avoid the mapped-buffer failure reproduced on WebGPU in Linux CI.
- Render workers receive referenced model dependencies instead of all imported files.
  Compressed textures are checked against decoded-memory budgets before allocation.
- Native MCP rendering stays responsive to transport keepalives and cancels abandoned work.
  Video duration reflects the encoded frame count, including short clips.
- Catalog body finishes preserve glass, metal, fabric and other secondary materials;
  explicit colors apply consistently. Bathroom basin interiors are visible through the
  countertop and cabinet openings.
- Model and plan-image caches refresh after source, texture or renderer changes. Cached
  PNG files are published atomically so readers cannot open a partially written image.
- Rendering and ceiling checks share the declared ceiling surface, including sloping
  walls, gables, partial roofs and storey elevation. Grouped lights retain their optics
  and respect source visibility in photos.
- Tilted box heights and roof fitting use transformed geometry. Wall fitting follows low
  eaves without producing spikes through the roof. Volume checks detect edge collisions
  with furniture, walls and door regions; joinery runs clip obstacles to the full height
  and depth band instead of probing only its center.
- Hosted opening measurements use the wall frame; explicit furniture orientation selects
  the correct wall face. Architectural warning acceptances stay independent when grouped.
- Browser CI tests isolate Chrome ports and profiles and clean up child processes.

### Scope

- The reviewed project is a separate artifact; the bundled browser demo has not been
  replaced. Remaining design and automation work is documented in
  `docs/AI-WORKFLOW-AUDIT.md`. Architectural feedback is not engineering certification.

## [1.4.3] - 2026-09-18

### Added

- In a browser the MCP comes up on its own. An editor whose point is that an AI drives it
  should not ask for a switch to be found first: a window wide enough to be somebody's
  desk is reachable the moment it opens, at the address it had last time. A phone is left
  alone — the address is of no use where there is nowhere to paste it.
- The browser opens on a home worth looking at: a corner classic seven of the kind the
  Upper West Side is full of — 164 m² with the gallery running the length of it, living
  and dining to the north, the bedrooms and the service wing to the south, furnished
  down to the lamps, with every room lit to the lux its use asks for and five points of
  view saved to walk through. It was drawn with the program's own tools over MCP, and
  the layout check passes on it.

### Fixed

- **The address stops being invalidated.** It is the tab's own from now on: the room is
  kept in the browser and asked for again on every connection, so switching off and on,
  reloading, closing the tab for the night — or the relay restarting under it — all give
  back the same address. What is registered in an AI client once goes on working.
- The commands the panel hands out no longer fail with "already exists": `claude mcp add`
  refuses a name it already has, and anybody who installed the app has one, so the line
  removes it first.
- The panel is one screen again: the state and its address, the clients as tabs with the
  line to copy under them, and the calls as they land — no numbered steps between
  somebody and the thing they came for.

## [1.4.2] - 2026-09-18

### Added

- The AI has the loudest button in the window: a pill at the head of the top bar that
  says which of three things is true — nothing can reach this window, waiting for your
  AI, or the name of the one that is connected — and pulses as it works. Not on a phone,
  where the row has no room and the status bar already says it.
- The panel behind it was rebuilt around three cards: where the window stands (with the
  address in a well of its own and one button to copy it), the three steps with the
  clients as chips and the command in its own block, and the last calls as a clean list.

### Fixed

- A reload no longer costs the connection. The tab writes down the room it holds and
  walks back into it when the page opens, so the address already pasted into an AI client
  keeps working; the relay holds a room a quarter of an hour for its tab to come back,
  and a room that is gone is forgotten rather than pointed at.

## [1.4.1] - 2026-09-18

### Fixed

- The browser's MCP address stopped answering after a couple of quiet minutes: what
  sits in front of the relay closes a WebSocket that says nothing for about a hundred
  seconds, and from the outside that looks exactly like a tab that went away. The tab
  is pinged every thirty seconds, so an address lasts as long as the tab does.

## [1.4.0] - 2026-09-18

The editor answers to an AI from a browser tab, with nothing installed, and is
usable on a phone.

### Added

- Your AI can drive the editor **in a browser, with nothing installed**. A tab cannot
  listen on a port, so it holds a connection outwards instead: switch the MCP on in the
  AI panel (or Ctrl+Shift+M) and the tab gets an address to paste into Claude Code, Codex,
  Cursor or anything else that speaks MCP. A small relay (`newera-relay`, in this
  repository) passes the calls and stores nothing — the project never leaves the tab, the
  address stops answering the moment it is switched off or the tab is closed, and one
  room's secrets are useless on another. `render_photo` and `video` are not offered there:
  they run for minutes on a CPU and would freeze the window.
- A place in the window that shows the MCP and proves it works: the **AI** menu and a chip
  in the status bar, a panel with the address, the snippet for each client — with a button
  that registers it for you when that client's command line is installed — a sentence to
  ask, and the last twelve tool calls as they land. The window names the client that
  connected and says each tool as it is used, so a working setup and a typo no longer look
  the same.
- The AI has a place in the top bar: a robot with the name of the thing beside it, lit
  when an agent is connected and pulsing on each call, one tap from the panel that says
  how to bring one. It is the first thing in the row, which is what this editor is for.
- Made for a phone. The editor sets its scale to what the screen can hold and, below 720
  points, shows one thing at a time — catalogue, plan or 3D — with a bar in thumb's reach
  and Ctrl+1/2/3 for whoever has a keyboard; the toolbar scrolls instead of falling off the
  edge. On the site every tappable thing gets the 44 px a thumb actually hits and the small
  print goes up a step. `scripts/mobile-audit.mjs` checks all of it at four real handset
  sizes, and CI fails on sideways scroll, a target too small or text too small to read.
- Every MCP tool can be called without a session (`newera_mcp::call`), which is what lets
  the browser answer for itself; a test walks the server's own tool list and fails if
  anything is unreachable, so the two cannot drift.

- The window speaks Spanish and French besides Portuguese and English: pick one in
  Help > Idioma / Language, or let it follow the system (`NEWERA_LANG`, `LC_ALL`,
  `LC_MESSAGES`, `LANG`, and the browser's language in the web editor) the first time
  it opens. The installers, the Linux desktop entry, the `.newera` file type and the
  macOS bundle speak the four as well.

- A visual identity of its own, taken from the site: a drafting studio in warm
  graphite with one blueprint blue to point with, day and night themes chosen in
  Ver › Tema (or left to the system), technical readouts set in mono, and the plan
  drawn on paper by day and on a lit board by night. Rulers, dialogs and the dimming
  behind them follow the theme; exports keep their own white paper.
- The left panel rebuilt: one search well, bands that open on a click anywhere on
  their line and count what is inside, filter keys per kind of element, and one row
  shape everywhere with the accent down the side of what is selected.
- What is picked on the plan lights up in the panel and is scrolled into view, with
  the band holding it opened.
- The catalog speaks the four languages too: every piece, every wall type and
  every finish is named in the language of the window, a piece dropped on the plan
  is created under that name, and the search finds it by the word on screen.
- The browser editor opens on a furnished demo home instead of an empty sheet, says in
  the visitor's language when a browser cannot start it, and is checked by CI in a real
  headless Chrome: it builds the page, draws a wall with the mouse and loads the viewer,
  failing on anything the page logs.
- It runs where there is no WebGPU: with wgpu's WebGL backend compiled in, a browser
  without WebGPU — Firefox today — falls back to WebGL 2 and still draws the plan and the
  3D view. The page says what happened, since Chrome and Edge are the supported browsers
  and Firefox is not guaranteed, and CI now draws the wall twice: once with WebGPU and
  once without.
- The site header fits a phone: the language switch and the GitHub star count moved into
  the menu instead of pushing the row off the screen.
- Found by search and by agents: schema.org data for the program and the questions,
  hreflang for the four languages, a sitemap, a robots.txt that welcomes the crawlers
  behind AI assistants, and an llms.txt that tells an agent what the program is and what
  it can be asked to do. The site header holds up on a phone, with the sections in a menu
  and the GitHub stars in the corner.
- The site opens in the visitor's own language — Portuguese, English, Spanish or
  French, read from the browser and overridable with `?lang=` or the switch in the
  corner — with the picture descriptions and the link preview to match.
- Full trackpad support: pinch to zoom, two fingers to move the plan or to turn the
  3D view (Shift to slide it), a twist to spin it, and momentum carried through —
  a wheel still zooms, told apart from a glide by how the machine reports it.

- The editor in the browser keeps what was drawn: the project is mirrored into the
  browser's own storage a moment after every change and opened again on the next visit,
  so closing a tab is no longer the same as throwing the drawing away. A test draws a
  wall, reloads the page and fails if it does not come back.
- The site, the editor and the viewer are published at <https://3dneweraai.com> — landing
  page at the root, editor at `/app/`, viewer at `/viewer/` — from one build script, with
  a workflow that puts them on Cloudflare Pages on every push.

### Fixed

- The browser editor stopped loading for anyone who had opened it before: the glue and the
  wasm keep the same names and a cache can hold one half of each build, which does not
  link. Both are stamped with the digest of the build now, so a cache can only ever serve a
  matching pair, and a mismatch that gets through anyway is fetched again past every cache.
- The row of plan versions no longer runs into the discipline, the layers and the storeys
  on a narrow screen: there the three go behind one button, with "compare versions" beside
  them, and the sheets scroll under it.
- Switching the browser's MCP off really closes it: the socket was kept inside the
  state, so a link that had fallen over left nothing to hang up and the old address went
  on answering while a new one was on screen. The socket is held beside the state now and
  closed whatever the state says.
- The console no longer fills with "Unable to preventDefault inside passive event
  listener": the editor's canvas and the viewer say they handle their own gestures, which
  is also what stops a phone scrolling the page while a finger drags the plan.
- The note about WebGL no longer covers the editor's own menus — on a phone that was most
  of the window — and goes on its own after it has been read.

- The window follows the language of the machine it runs on, which made the interface
  tests read English labels on a macOS runner set to `en_US` and fail the whole suite:
  under test it now speaks the language the code is written in, whatever the machine says.

- Dialogs read as part of the program: one width, a titled head, fields grouped under
  mono headings, a body that scrolls instead of running off the screen, and one footer
  with the action filled in the accent and the way out beside it.
- Texts that never had a translation — the tool names in the toolbar and the Plan
  menu, and the status bar lines about opening, saving, exporting and importing —
  now follow the chosen language instead of staying in Portuguese.

## [1.3.0] - 2026-09-16

The electrical, telecom and plumbing projects, checked against the text of the
norms, and a long round of fixes found by an agent working a real 65 m²
apartment through the MCP server.

### Added

- Electrical project: network, TV, Wi-Fi and telecom panel points in the catalog;
  circuits assigned per point or the whole division in one call (MCP `electrical`
  `assign`), a load schedule and the NBR 5410 checks, drawn on the plan with circuit
  numbers and opened from the discipline menu in the editor.
- The supply is suggested by Enel SP's categories (single-, two- or three-phase and its
  entry breaker); the panel is filled in DIN modules with DR, surge protector and the
  spare ways NBR 5410 asks, and says when it does not fit.
- Runs laid the way they are built: along walls, inside the slab or the floor, by the
  cheapest tree from the panel, with their bill of materials (conduit, boxes, wire per
  conductor, cable, connectors), voltage drop and conduit grouping. Cables drawn by hand
  are measured and replaced by the routed run.
- Adhesive flat wiring tape (Eletrofitas) as a way to lay power runs: the model by the
  load and by whether a socket needs earth, refused in wet rooms and over its rating,
  bought as Leroy Merlin kits with codes, prices and splices.
- Wi-Fi access points: coverage per room and band from walls, doors, windows, slabs and
  low-e glass; a suggested placement; standard, PoE, uplink and cable category checks.
- Automation devices (smart relay, smart switch, dimmer, presence sensor, smart lock),
  with their standby load and what each needs (neutral in the box, dimmer load, sensor
  height and reach).
- Outlets built into furniture: pop-up towers (60, 85 and 100 mm), desk boxes and panel
  outlets, modelled in 3D, seated on the top they go into and checked for edge distance,
  room below, sink and cooktop distance and the socket a plug-in tower needs.
- Plumbing project: cold, hot, sewer, vent and gas points; drains as the models they are
  (sizes, flow, fixed in the floor), vent branches routed, gas grilles drawn, and rules
  from NBR 8160, NBR 5626, NBR 13103 and São Paulo's sanitary code.
- Balcony guards told apart: iron railings, glass guards and glass closures, checked for
  height (1,10 m), bar gap (11 cm), laminated glass, and a closure that is no guard; MCP
  `place`/`update` take `glass` and `gap`.
- Lighting, appliances and joinery are layers of the plan, shown or hidden on the plan
  and in 3D; hiding the electrical project hides all of it, lamps included.
- Ergonomics: single beds side by side, accessible tops and beds, gas appliances in
  bathrooms and bedrooms; a fix says what it leaves behind, and two findings that cannot
  both fit say so.
- MCP: `move` takes `to=[x,y]`; `place` copies a piece already in the project;
  `cut_list` reads joinery drawn by hand; a part of a group can be renamed; findings of
  every discipline can be accepted with a reason, and an acceptance that outlived its
  finding is listed, pruned or pointed to the finding that took its place; deleting a
  piece names the labels left pointing at it; `update fixed` declares a piece fixed or
  free-standing when its name does not say.
- Crash reports to Sentry in released binaries, on by default and off from the Help menu.
- Agents can report what a tool could do better, with the whole case.

### Changed

- Every figure and citation was checked against the norm texts: NBR 5410 (lighting load
  by area, kitchen and bathroom outlets, DR, grouping, 4 % drop), NBR 16264, NBR 8160,
  NBR 5626, NBR 13103, NBR 15575-1 annex F, NBR 9050, NBR 14718, NBR 7199 and São
  Paulo's building code (Decreto 57.776) — see `docs/NORMAS.md`.

### Fixed

- A fixed point (outlet, switch, RJ45, Wi-Fi point, drain) is set on its structure —
  wall, ceiling, floor or the top it is built into — never loose in a room, on glass, in
  a door or window span, too close to a frame or inside an appliance; the refusal names
  the nearest free place.
- A piece is classified by what it is, not by a word in its name: a cabinet or countertop
  named after the appliance beside it is joinery, cabinet parts named after the sink are
  not sinks, and "Lavanderia" is no "lava".
- A reply names every change it made, and a change that changes nothing says so with the
  value already there; `update` refuses a field that does not exist.
- Groups resized keep their boards and the face they declare; dimensions held by a part
  follow it; reference numbers stay with their piece; a door nudged along its wall keeps
  its side.
- A painted wall no longer covers the door in it; the load schedule's findings scroll.
- The repository checks out on Windows again (`scripts/aux` was a reserved name there).

## [1.2.0] - 2026-09-15

### Changed

- A rule advises, it never refuses to draw: what the workshop would say travels with the
  drawing in `notes`, and a request that cannot be built as asked builds the nearest thing
  and says so. Only what has no geometry at all fails.

### Added

- The editor says what long work is doing, and it can be walked away from.
- `faces` is read from what was built into a piece (doors, drawer fronts, kick), a
  dimension that holds onto what it marks is measured again on every change, findings carry
  their `weight`, and `dry` can answer `"summary"`.

### Fixed

- An appliance that stopped fitting its niche is reported by `check_layout`.
- Walls join where they touch, not only where they end.
- Light and air cross rooms open to each other and through glass; notes that no longer
  match their piece are reported by `stale`.

## [1.1.0] - 2026-09-15

### Added

- Landing page on GitHub Pages, in Portuguese and English.
- The mark as a real icon set on macOS, Windows and Linux; installers speak the user's
  language.
- MCP answers what an agent actually asks: measure, classify, preview.

### Changed

- The MCP tools moved out of one file into a module per tool, behind a test that freezes
  the tool surface.

### Fixed

- `cargo run -p newera` picks the editor binary.
- Installers find the latest release without the GitHub API.

## [1.0.0] - 2026-09-14

First release: the desktop editor, the browser editor and viewer, the HTTP API and the
MCP server, with everything below.

### Added

- Downloads for Windows, macOS (Apple Silicon and Intel) and Linux on every release, with
  one-line installers that need no administrator: `install-windows.ps1` (Start menu and
  desktop shortcuts, user `PATH`, listed in Settings > Apps) and `install-macos.sh`
  (`3D New Era AI.app`, ad-hoc signed; builds from source when there is no binary).
  Both skip what is already installed, clean up after themselves and register the MCP
  server in Claude Code and Codex. The README shows how to connect Gemini CLI, VS Code,
  Cursor, Windsurf, Claude Desktop and MCP apps for other models. `newera-gui.exe` opens the editor without a console window.

- Smaller `.newera` files: every project is a deflated bundle with compact JSON, and large
  JPEG textures are recompressed once on save (quality 85, at most 2048 px). The showcase
  apartment went from 12.3 MB to 2.7 MB. Plain JSON projects from earlier saves still open.

- MCP `lighting`: lux per room by photometry against NBR ISO/CIE 8995-1; `fill` places the
  fixtures a room needs. Photos get a half-strength white balance and render on half the
  cores by default (`NEWERA_RENDER_THREADS`).

- Smart guides while moving pieces on the plan: edges and centers line up with other pieces,
  rooms and wall faces (dashed lines show what they lined up with), equal gaps between two
  neighbors, and live distances to what is around. Shift turns the magnet off.

- Walls, glass and panels fitted to the roof above them (MCP `fit_roof`, Planta > Ajustar ao
  telhado): under an A-frame a partition is split at the ridge with sloping tops and a glass
  gable becomes a triangle; they keep following the roof in the same undo step.

- MCP `embed`: a sink bowl or cooktop set into a joinery countertop with a cutout sized
  from the real item, or an oven, microwave or other appliance into a cabinet niche (boards
  around it, doors above and below, several niches per tower). The item becomes part of the
  host and follows it through moves, edits and cabinet runs; errors give the size to use.
  New catalog items: cooktop, sink bowl and built-in oven.

- "Armários na parede…" in the editor (Planta menu, one wall selected): base, wall or tall
  row, sink and cooktop positions, front color and drawer units, with a preview of the
  modules before building them. The planning moved into `newera-joinery`, shared with MCP.

- "Ergonomia…" window in the editor (Planta menu): the habitability score and findings for
  the occupants you set, kept current while you edit; a finding selects what it is about and
  its checked fix applies in one click.

- MCP `ergonomics`: a review of the plan for the people who live there (occupants, children,
  elderly, wheelchair, the cook's height). Room to walk beside beds and in front of the stove,
  beds, seats, bathrooms and wardrobes per person, the kitchen triangle and heights, doors that
  hit furniture, ceiling heights, windows, minimum furniture and wheelchair turning space, each
  finding with its numbers, the fix and the Brazilian reference (NBR 9050, NBR 15575-1, IBGE).

- MCP `cabinet_run` fills a wall with cabinets sized for it: the free stretches between
  corners, doors (and their swing), windows, fridge and stove become even modules with no
  useless leftovers — pull-outs for 15–30 cm, fillers below — a drawer unit by the stove,
  countertops, a cabinet over the fridge, a gap for the hood, and blind corner modules where
  two runs meet in an L (the other wall is planned again in the same undo step).

- Joinery for AI-driven design: MCP `joinery` builds modular cabinets, slatted panels,
  countertops with sink/cooktop cutouts, plaster coves with LED, shadow gaps and modular
  sofas from flat parameters, edits them incrementally by id and explains broken workshop
  rules with the value to use; `cut_list` exports CSV and DXF/SVG sheet layouts.

- Plugins (external programs over the HTTP API) and multi-user sessions with presence,
  named cursors on the plan and conflict detection.

- Full editor in the browser on WebGPU (`make web-editor`, `/editor/`).

- Live sun in the 3D view from the compass and hour; labels and dimensions shown in
  3D (dialogs and MCP `in3d`/`elev`/`pitch`).

- Video along the camera path (Catmull-Rom, Motion-JPEG AVI): "Criar vídeo" window
  and MCP `video`.

- English interface (Ajuda › Idioma / Language).

- Web viewer in WebAssembly (`web/`, `make web-serve`).

- Symbol legend for electrical/plumbing; sloping walls, baseboards and elevated
  polylines in 3D.

- Server: token auth for network exposure, SSE revision events, REST commands endpoint.

- Furniture top views on the plan (Ver › Móveis na planta), rendered in the background.

- Photo renderer (path tracing with sun, lamps, glass, denoising): app window and MCP
  `render_photo`.

- Exports: glTF binary and OBJ/MTL 3D models, vector PDF plans at scale.

- `newera-render`: software 3D renderer; MCP `render_3d` and REST `/api/view.png`;
  precise dimension magnet; dimensions by intent over MCP; compacted tool schemas.

- Engineering dimension chains and room reference schedules (tags, sizes, brand/model/
  link) drawn on the plan; MCP `annotations`.

- Electrical and plumbing projects over the plan (disciplines, symbols, line tool,
  quantities, MCP `disciplines`); points of view with visitor camera and MCP `cameras`;
  polyline and label style editing; groups carry their pieces.

- Sweet Home 3D import (`.sh3d`) with a lossless model: polylines, text styles, cameras,
  environment, print settings, furniture metadata, groups, lights, sashes, wall
  cut-outs, level layouts; project bundles (`.newera` ZIP with assets).

- Wall types (drywall, masonry, concrete, glass…) and surface finishes for wall sides,
  floors and ceilings: procedural GPU patterns and image textures at real size; room
  ceilings in 3D; MCP `materials` and `type`/`left`/`right`/`sides`/`floor_mat`/`ceil_mat`.

- Levels: storeys with elevation/height/slab, level selector, faint reference of the
  storey below, stacked 3D with stair openings; MCP `levels`.

- Plan variants as tabs: duplicate (Ctrl+T), switch (Ctrl+Tab), rename, close, compare
  versions; project format v2; MCP `variants`.

- M2 furniture at real scale: parametric catalog (55 items, 3D + plan symbols), OBJ/glTF
  import, catalog panel with search, placement with ghost, rotate/resize handles, doors
  and windows cutting walls, layout checks, furniture dialog; MCP `catalog`, `place`,
  `check_layout`.

- M1 working drawings: background image calibration, arc walls, wall splitting and
  endpoint handles, dimensions, labels, room detection from walls, magnetism with typed
  lengths, rulers, units, selection with move/copy/paste, project files and plan export.
- `newera-draw`: one plan scene for the editor, PNG renders (MCP `render_plan`) and SVG.
- Consolidated, token-lean MCP tools (`create`, `update`, `move`, …).
- UI interaction tests with `egui_kittest`.

- Workspace with `newera-core`, `newera-mcp`, `newera-server`, `newera-app` and the `newera` binary.
- Reversible commands with undo/redo, atomic batches and document revisions.
- Walls, rooms and compass; short typed ids (`w12`, `r3`) and compact `[x, y]` points.
- Desktop editor in the Sweet Home 3D layout: catalog and home tree, 2D floor plan, native 3D view.
- MCP server (stdio and Streamable HTTP) with token-efficient tools; REST `GET /api/home`.
- Wall joins (miters, T/X junctions) and concave room floors, shared by plan and 3D.
- `scripts/mcp.sh` / `make mcp` to call MCP tools from the shell.
- CI for formatting, clippy, tests on Linux/macOS/Windows, MCP smoke test, MSRV and cargo-deny; release builds.

[Unreleased]: https://github.com/leandrodaf/3d-new-era-ai/compare/v3.2.0...HEAD
[3.2.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v3.1.0...v3.2.0
[3.1.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v3.0.0...v3.1.0
[3.0.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v2.0.0...v3.0.0
[2.0.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.10.0...v2.0.0
[1.10.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.9.2...v1.10.0
[1.9.2]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.9.1...v1.9.2
[1.9.1]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.9.0...v1.9.1
[1.9.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.8.0...v1.9.0
[1.8.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.7.0...v1.8.0
[1.7.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.6.0...v1.7.0
[1.6.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.5.0...v1.6.0
[1.5.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.4.3...v1.5.0
[1.4.3]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.4.2...v1.4.3
[1.4.2]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.4.1...v1.4.2
[1.4.1]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.4.0...v1.4.1
[1.4.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.2.0...v1.3.0
[1.2.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/leandrodaf/3d-new-era-ai/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/leandrodaf/3d-new-era-ai/releases/tag/v1.0.0
