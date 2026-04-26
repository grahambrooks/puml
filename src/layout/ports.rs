//! Shared port selection for orthogonal edge routing.
//!
//! A "port" is the point on a node's bounding box where an edge attaches.
//! Picking the right port (top, bottom, left, right) determines which side
//! the route exits and what shape the resulting polyline takes.
//!
//! The default rule: choose whichever axis (vertical or horizontal) has the
//! greater separation between the two nodes' centres. That puts the port on
//! the side facing the other node — which is what a human reader would
//! draw, and what `orthogonal_through_ports` then turns into a clean Z or
//! L bend.
//!
//! For decision diamonds the bounding-box edge midpoints coincide with the
//! diamond's four corner points, so the same axis-of-greater-separation
//! rule yields the conventional top-entry / side-exit attachment without
//! any shape-specific code.

use super::sugiyama::Side;

/// Pick the port on bounding box `bbox = (x, y, w, h)` facing point
/// `toward = (tx, ty)`.
///
/// Returns the port coordinate and which side of the box it sits on.
/// `vertical_only` forces top/bottom selection — useful for thin
/// horizontal shapes (fork/join bars, swimlane separators) where a left or
/// right attachment would land on a 0-width edge.
pub fn pick_port(
    bbox: (f64, f64, f64, f64),
    toward: (f64, f64),
    vertical_only: bool,
) -> ((f64, f64), Side) {
    let (x, y, w, h) = bbox;
    let cx = x + w / 2.0;
    let cy = y + h / 2.0;
    let (tx, ty) = toward;
    let dx = tx - cx;
    let dy = ty - cy;
    let vertical = vertical_only || dy.abs() >= dx.abs();
    if vertical {
        if dy < 0.0 {
            ((cx, y), Side::Top)
        } else {
            ((cx, y + h), Side::Bottom)
        }
    } else if dx < 0.0 {
        ((x, cy), Side::Left)
    } else {
        ((x + w, cy), Side::Right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_bottom_when_target_below() {
        let ((x, y), side) = pick_port((0.0, 0.0, 100.0, 50.0), (50.0, 200.0), false);
        assert_eq!((x, y), (50.0, 50.0));
        assert_eq!(side, Side::Bottom);
    }

    #[test]
    fn picks_top_when_target_above() {
        let ((x, y), side) = pick_port((0.0, 100.0, 100.0, 50.0), (50.0, 0.0), false);
        assert_eq!((x, y), (50.0, 100.0));
        assert_eq!(side, Side::Top);
    }

    #[test]
    fn picks_right_when_target_right_and_horizontal_dominates() {
        let ((x, y), side) = pick_port((0.0, 0.0, 100.0, 50.0), (300.0, 25.0), false);
        assert_eq!((x, y), (100.0, 25.0));
        assert_eq!(side, Side::Right);
    }

    #[test]
    fn picks_left_when_target_left_and_horizontal_dominates() {
        let ((x, y), side) = pick_port((100.0, 0.0, 100.0, 50.0), (0.0, 25.0), false);
        assert_eq!((x, y), (100.0, 25.0));
        assert_eq!(side, Side::Left);
    }

    #[test]
    fn ties_resolve_to_vertical() {
        // Centre (50, 50), target (150, 150) → dx == dy → vertical wins
        // (the >= comparison favours vertical on ties).
        let ((_, _), side) = pick_port((0.0, 0.0, 100.0, 100.0), (150.0, 150.0), false);
        assert_eq!(side, Side::Bottom);
    }

    #[test]
    fn vertical_only_overrides_horizontal_dominance() {
        // Target sits to the right and slightly down. Without the override
        // the port would be Right; with vertical_only it must be Bottom.
        let ((x, y), side) = pick_port((0.0, 0.0, 100.0, 10.0), (300.0, 50.0), true);
        assert_eq!((x, y), (50.0, 10.0));
        assert_eq!(side, Side::Bottom);
    }
}
