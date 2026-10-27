//! Room detection: the enclosed space around a point, bounded by the inner
//! faces of the walls (like double-clicking inside a room in a CAD tool).

use geo::{Contains, Coord, LineString, Polygon};

use crate::elements::Wall;
use crate::geometry::{Point2, signed_area};
use crate::joins::wall_outlines;

/// Polygon of the space enclosed by walls that contains `point`, or `None`
/// when the point is not fully surrounded (open walls, outside, or on a wall).
/// Like [`detect_room`], with room dividers (see
/// [`crate::Polyline::room_divider`]) closing open spaces as if they were walls.
pub fn detect_room_with_dividers(
    walls: &[Wall],
    dividers: &[&crate::Polyline],
    point: Point2,
) -> Option<Vec<Point2>> {
    if dividers.is_empty() {
        return detect_room(walls, point);
    }
    let mut all = walls.to_vec();
    for line in dividers {
        let mut points = line.points.clone();
        if line.closed && points.len() > 2 {
            points.push(points[0]);
        }
        for pair in points.windows(2) {
            if pair[0].distance(pair[1]) > 0.1 {
                let mut wall = Wall::new(crate::ids::WallId(u64::MAX), pair[0], pair[1]);
                wall.thickness = 1.0;
                all.push(wall);
            }
        }
    }
    detect_room(&all, point)
}

/// A point well inside a polygon (its centroid when that is inside, else the
/// center of its largest triangle).
pub fn interior_point(points: &[Point2]) -> Option<Point2> {
    if points.len() < 3 {
        return None;
    }
    let polygon = Polygon::new(ring(points), vec![]);
    if let Some(c) = geo::Centroid::centroid(&polygon)
        && polygon.contains(&c)
    {
        return Some(Point2::new(c.x(), c.y()));
    }
    crate::geometry::triangulate(points)
        .into_iter()
        .map(|[a, b, c]| (points[a], points[b], points[c]))
        .max_by(|x, y| {
            let area = |(a, b, c): &(Point2, Point2, Point2)| {
                ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs()
            };
            area(x).total_cmp(&area(y))
        })
        .map(|(a, b, c)| Point2::new((a.x + b.x + c.x) / 3.0, (a.y + b.y + c.y) / 3.0))
}

/// New outlines for the rooms that follow their walls ([`crate::Room::auto`])
/// whose walls or dividers changed shape. Rooms left without an enclosure keep
/// their outline, and so does one that would come out the same as a room
/// before it (see [`rooms_sharing_a_space`]).
pub fn rooms_following_walls(home: &crate::Home) -> Vec<crate::Room> {
    follow(home).0
}

/// Rooms following their walls that no longer have a space of their own: the
/// wall between two of them moved away, and both would be detected as the
/// same polygon. Each pair is `(room, other)`; `room` comes later in the list
/// and keeps the outline it had instead of turning into a copy of `other`.
pub fn rooms_sharing_a_space(home: &crate::Home) -> Vec<(crate::RoomId, crate::RoomId)> {
    follow(home).1
}

fn follow(home: &crate::Home) -> (Vec<crate::Room>, Vec<(crate::RoomId, crate::RoomId)>) {
    let same = |a: &[Point2], b: &[Point2]| {
        a.len() == b.len() && a.iter().zip(b).all(|(p, q)| p.distance(*q) < 0.05)
    };
    let mut taken: Vec<(Option<crate::LevelId>, crate::RoomId, Vec<Point2>)> = Vec::new();
    let (mut moved, mut sharing) = (Vec::new(), Vec::new());
    for room in home.rooms.iter().filter(|room| room.auto) {
        let level = home.resolve_level(room.level);
        let view = home.level_view(room.level);
        let dividers: Vec<&crate::Polyline> =
            view.polylines.iter().filter(|p| p.room_divider).collect();
        let Some(points) = interior_point(&room.points)
            .and_then(|inside| detect_room_with_dividers(&view.walls, &dividers, inside))
        else {
            continue;
        };
        if let Some((_, other, _)) = taken
            .iter()
            .find(|(l, _, taken)| *l == level && same(taken, &points))
        {
            sharing.push((room.id, *other));
            continue;
        }
        taken.push((level, room.id, points.clone()));
        if !same(&points, &room.points) {
            moved.push(crate::Room {
                points,
                ..room.clone()
            });
        }
    }
    (moved, sharing)
}

pub fn detect_room(walls: &[Wall], point: Point2) -> Option<Vec<Point2>> {
    let polygons: Vec<Polygon<f64>> = wall_outlines(walls)
        .into_iter()
        .filter(|outline| outline.len() >= 3 && signed_area(outline).abs() > 1e-6)
        .map(|outline| Polygon::new(ring(&outline), vec![]))
        .collect();
    if polygons.is_empty() {
        return None;
    }
    let union = geo::unary_union(&polygons);
    let target = geo::Point::new(point.x, point.y);

    // Holes in the union of all wall footprints are exactly the enclosed rooms.
    // Nested holes can't happen (a hole can't contain walls of the same
    // polygon), but separate buildings can, so pick the smallest match.
    union
        .iter()
        .flat_map(|poly| poly.interiors().iter())
        .filter(|hole| Polygon::new((*hole).clone(), vec![]).contains(&target))
        .map(simplify)
        .min_by(|a, b| signed_area(a).abs().total_cmp(&signed_area(b).abs()))
}

fn ring(points: &[Point2]) -> LineString<f64> {
    let mut coords: Vec<Coord<f64>> = points.iter().map(|p| Coord { x: p.x, y: p.y }).collect();
    coords.push(coords[0]);
    LineString::new(coords)
}

/// Ring to open polygon, dropping duplicate and collinear vertices and
/// rounding to 0.01 cm to hide floating-point noise from the boolean ops.
fn simplify(ring: &LineString<f64>) -> Vec<Point2> {
    let round = |v: f64| (v * 100.0).round() / 100.0;
    let mut points: Vec<Point2> = ring
        .coords()
        .map(|c| Point2::new(round(c.x), round(c.y)))
        .collect();
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    points.dedup();
    let mut changed = true;
    while changed && points.len() > 3 {
        changed = false;
        let n = points.len();
        if let Some(i) = (0..n).find(|&i| {
            let (a, b, c) = (points[(i + n - 1) % n], points[i], points[(i + 1) % n]);
            let cross = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
            cross.abs() < 1e-6 * a.distance(c).max(1.0)
        }) {
            points.remove(i);
            changed = true;
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::polygon_area;
    use crate::ids::WallId;

    fn walls_from(polyline: &[(f64, f64)], closed: bool, thickness: f64) -> Vec<Wall> {
        let mut points: Vec<Point2> = polyline.iter().map(|&(x, y)| Point2::new(x, y)).collect();
        if closed {
            points.push(points[0]);
        }
        points
            .windows(2)
            .enumerate()
            .map(|(i, pair)| Wall {
                thickness,
                ..Wall::new(WallId(i as u64 + 1), pair[0], pair[1])
            })
            .collect()
    }

    #[test]
    fn detects_inner_faces_of_a_closed_square() {
        let walls = walls_from(
            &[(0.0, 0.0), (400.0, 0.0), (400.0, 300.0), (0.0, 300.0)],
            true,
            20.0,
        );
        let room = detect_room(&walls, Point2::new(200.0, 150.0)).expect("room");
        assert_eq!(room.len(), 4);
        assert!(
            (polygon_area(&room) - 380.0 * 280.0).abs() < 0.1,
            "{}",
            polygon_area(&room)
        );
    }

    #[test]
    fn open_walls_or_outside_points_detect_nothing() {
        let open = walls_from(
            &[(0.0, 0.0), (400.0, 0.0), (400.0, 300.0), (0.0, 300.0)],
            false,
            20.0,
        );
        assert!(detect_room(&open, Point2::new(200.0, 150.0)).is_none());
        let closed = walls_from(
            &[(0.0, 0.0), (400.0, 0.0), (400.0, 300.0), (0.0, 300.0)],
            true,
            20.0,
        );
        assert!(detect_room(&closed, Point2::new(900.0, 150.0)).is_none());
    }

    #[test]
    fn partition_wall_splits_the_space_even_without_shared_endpoints() {
        let mut walls = walls_from(
            &[(0.0, 0.0), (600.0, 0.0), (600.0, 400.0), (0.0, 400.0)],
            true,
            20.0,
        );
        // Partition touching the outer walls' middles (T junctions by overlap).
        walls.push(Wall {
            thickness: 10.0,
            ..Wall::new(
                WallId(99),
                Point2::new(250.0, 0.0),
                Point2::new(250.0, 400.0),
            )
        });
        let left = detect_room(&walls, Point2::new(100.0, 200.0)).expect("left room");
        let right = detect_room(&walls, Point2::new(400.0, 200.0)).expect("right room");
        assert!(
            (polygon_area(&left) - 235.0 * 380.0).abs() < 0.1,
            "{}",
            polygon_area(&left)
        );
        assert!(
            (polygon_area(&right) - 335.0 * 380.0).abs() < 0.1,
            "{}",
            polygon_area(&right)
        );
    }
}

#[cfg(test)]
mod follow_tests {
    use crate::{Command, Document, Point2, Polyline, PolylineId, Room, Wall, ops};

    fn square(doc: &mut Document, w: f64, h: f64) {
        let pts = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
        let commands = (0..4)
            .map(|i| {
                let (a, b) = (pts[i], pts[(i + 1) % 4]);
                Command::insert(Wall::new(
                    doc.new_wall_id(),
                    Point2::new(a.0, a.1),
                    Point2::new(b.0, b.1),
                ))
            })
            .collect();
        doc.execute(Command::Batch { commands }).unwrap();
    }

    fn auto_room(doc: &mut Document, at: Point2) {
        let home = doc.home();
        let dividers: Vec<&Polyline> = home.polylines.iter().filter(|p| p.room_divider).collect();
        let points = super::detect_room_with_dividers(&home.walls, &dividers, at).unwrap();
        let mut room = Room::new(doc.new_room_id(), "Sala", points);
        room.auto = true;
        doc.execute(Command::insert(room)).unwrap();
    }

    #[test]
    fn detected_rooms_follow_their_walls_in_the_same_undo_step() {
        let mut doc = Document::default();
        square(&mut doc, 400.0, 300.0);
        auto_room(&mut doc, Point2::new(200.0, 150.0));
        let area = |doc: &Document| doc.home().rooms[0].area();
        assert!((area(&doc) - 385.0 * 285.0).abs() < 1.0);

        // Push the right wall 100 cm further: the room grows with it.
        let right = doc.home().walls[1].id;
        ops::translate(&mut doc, &[right.into()], 100.0, 0.0, true).unwrap();
        assert!((area(&doc) - 485.0 * 285.0).abs() < 1.0, "{}", area(&doc));

        // One undo brings back wall and room together.
        doc.undo().unwrap();
        assert!((area(&doc) - 385.0 * 285.0).abs() < 1.0, "{}", area(&doc));
        doc.redo().unwrap();
        assert!((area(&doc) - 485.0 * 285.0).abs() < 1.0);

        // A room drawn by hand keeps its outline.
        let mut manual = doc.home().rooms[0].clone();
        manual.auto = false;
        doc.execute(Command::update(manual)).unwrap();
        ops::translate(&mut doc, &[right.into()], -100.0, 0.0, true).unwrap();
        assert!((area(&doc) - 485.0 * 285.0).abs() < 1.0);
    }

    #[test]
    fn two_rooms_left_in_one_space_do_not_become_copies() {
        let mut doc = Document::default();
        square(&mut doc, 600.0, 300.0);
        let partition = doc.new_wall_id();
        doc.execute(Command::insert(Wall::new(
            partition,
            Point2::new(300.0, 0.0),
            Point2::new(300.0, 300.0),
        )))
        .unwrap();
        auto_room(&mut doc, Point2::new(150.0, 150.0));
        auto_room(&mut doc, Point2::new(450.0, 150.0));
        let right = doc.home().rooms[1].points.clone();

        // The partition goes out of the house: one space is left.
        ops::translate(&mut doc, &[partition.into()], 0.0, 400.0, true).unwrap();
        let home = doc.home();
        assert!(
            (home.rooms[0].area() - 585.0 * 285.0).abs() < 1.0,
            "the first room takes the space: {}",
            home.rooms[0].area()
        );
        assert_eq!(
            home.rooms[1].points, right,
            "the second keeps its outline instead of copying the first"
        );
        assert_eq!(
            super::rooms_sharing_a_space(home),
            vec![(home.rooms[1].id, home.rooms[0].id)]
        );
    }

    #[test]
    fn dividers_split_open_spaces_and_rooms_follow_them() {
        let mut doc = Document::default();
        square(&mut doc, 600.0, 300.0);
        let mut line = Polyline::new(
            PolylineId(0),
            vec![Point2::new(300.0, 0.0), Point2::new(300.0, 300.0)],
        );
        line.id = doc.new_polyline_id();
        line.room_divider = true;
        let divider = line.id;
        doc.execute(Command::insert(line)).unwrap();
        auto_room(&mut doc, Point2::new(150.0, 150.0));
        auto_room(&mut doc, Point2::new(450.0, 150.0));
        let areas: Vec<f64> = doc.home().rooms.iter().map(Room::area).collect();
        assert!(
            areas.iter().all(|a| (a - areas[0]).abs() < 400.0),
            "{areas:?}"
        );
        assert!(areas[0] < 300.0 * 290.0, "{areas:?}");

        // Moving the divider resizes both rooms.
        ops::translate(&mut doc, &[divider.into()], 100.0, 0.0, true).unwrap();
        let areas: Vec<f64> = doc.home().rooms.iter().map(Room::area).collect();
        assert!(areas[0] > areas[1] + 50.0 * 280.0, "{areas:?}");
    }
}
