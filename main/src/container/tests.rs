#[cfg(test)]
mod tests {
    use crate::container::axis::Axis;
    use crate::container::{Container, Window, spread_bounds_along_axis};
    use core_graphics::{Bounds, WindowId};
    use std::collections::HashSet;

    const EPSILON: f64 = 1e-10;

    const fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < EPSILON
    }

    fn dummy_bounds() -> Bounds {
        Bounds {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        }
    }

    fn dummy_window(id: WindowId) -> Window {
        Window {
            id,
            min_width: 100.0,
            min_height: 100.0,
        }
    }

    fn dummy_empty() -> Container {
        Container::Empty {
            bounds: dummy_bounds(),
        }
    }

    fn dummy_leaf(window_id: WindowId) -> Container {
        Container::Leaf {
            bounds: dummy_bounds(),
            padding: 0.0,
            window: dummy_window(window_id),
        }
    }

    // This is not a valid `Container` since the child bounds are wrong. It only
    // servers to be used in tests that are not checking correctness of bounds.
    fn dummy_split(axis: Axis, window_ids: &[WindowId]) -> Container {
        Container::Split {
            bounds: dummy_bounds(),
            padding: 0.0,
            axis,
            children: window_ids.iter().map(|id| dummy_leaf(*id)).collect(),
        }
    }

    #[test]
    fn get_bounds() {
        let empty = dummy_empty();
        let leaf = dummy_leaf(WindowId::from(1u32));
        let split = dummy_split(Axis::default(), &[WindowId::from(1u32)]);

        assert_eq!(empty.get_bounds(), dummy_bounds());
        assert_eq!(leaf.get_bounds(), dummy_bounds());
        assert_eq!(split.get_bounds(), dummy_bounds());
    }

    #[test]
    fn contains_window() {
        assert!(!dummy_empty().contains_window(WindowId::from(1u32)));

        let target = WindowId::from(1u32);
        let leaf_with = dummy_leaf(target);
        let leaf_without = dummy_leaf(WindowId::from(2u32));
        let split_with = dummy_split(Axis::Horizontal, &[target]);
        let split_without = dummy_split(Axis::Horizontal, &[WindowId::from(2u32)]);

        assert!(leaf_with.contains_window(target));
        assert!(!leaf_without.contains_window(target));
        assert!(split_with.contains_window(target));
        assert!(!split_without.contains_window(target));
    }

    #[test]
    fn getting_window_bounds_from_non_leaf_is_none() {
        let empty = dummy_empty();
        assert!(empty.window_bounds().is_none());

        let split = dummy_split(Axis::default(), &[]);
        assert!(split.window_bounds().is_none());
    }

    #[test]
    fn leaf_window_bounds_includes_correct_padding() {
        let container_bounds = dummy_bounds();
        const PADDING: f64 = 10.0;
        let exp_window_bounds = Bounds {
            height: container_bounds.height - 20.0,
            width: container_bounds.width - 20.0,
            x: container_bounds.x + PADDING,
            y: container_bounds.y + PADDING,
        };

        let container = Container::Leaf {
            bounds: container_bounds,
            padding: PADDING,
            window: dummy_window(WindowId::from(1u32)),
        };

        assert_eq!(container.window_bounds().unwrap(), exp_window_bounds);
    }

    #[test]
    fn add_window_to_empty_creates_leaf() {
        let mut container = dummy_empty();
        let window_id = WindowId::from(1u32);

        assert!(container.add_window(dummy_window(window_id), 10.0).is_ok());

        match container {
            Container::Split { children, .. } => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    Container::Leaf {
                        window,
                        padding,
                        bounds,
                    } => {
                        assert_eq!(window.id, window_id);
                        assert_eq!(*padding, 10.0);
                        assert_eq!(*bounds, dummy_bounds());
                    }
                    _ => panic!("Expected a leaf inside split"),
                }
            }
            _ => panic!("Expected container to become a split"),
        }
    }

    #[test]
    fn add_window_to_split_adds_new_leaf() {
        let container_bounds = dummy_bounds();
        let mut container = dummy_empty();

        // The split should be horizontal with each leaf taking half the space.
        let exp_first_leaf_bounds = Bounds {
            height: container_bounds.height,
            width: container_bounds.width / 2.0,
            x: 0.0,
            y: 0.0,
        };
        let exp_second_leaf_bounds = Bounds {
            height: container_bounds.height,
            width: container_bounds.width / 2.0,
            x: container_bounds.width / 2.0,
            y: 0.0,
        };

        let first_id = WindowId::from(1u32);
        let second_id = WindowId::from(2u32);

        const PADDING: f64 = 0.0;

        container
            .add_window(dummy_window(first_id), PADDING)
            .unwrap();
        container
            .add_window(dummy_window(second_id), PADDING)
            .unwrap();

        match container {
            Container::Split { children, .. } => {
                assert_eq!(children.len(), 2);
                let ids: Vec<_> = children
                    .iter()
                    .map(|c| {
                        if let Container::Leaf { window, .. } = c {
                            window.id
                        } else {
                            WindowId::from(0u32)
                        }
                    })
                    .collect();
                assert!(ids.contains(&first_id));
                assert!(ids.contains(&second_id));
                assert_eq!(children[0].get_bounds(), exp_first_leaf_bounds);
                assert_eq!(children[1].get_bounds(), exp_second_leaf_bounds);
            }
            _ => panic!("Expected container to be a split"),
        }
    }

    #[test]
    fn add_window_to_leaf_errors() {
        let mut container = dummy_leaf(WindowId::from(1u32));
        let result = container.add_window(dummy_window(WindowId::from(2u32)), 10.0);
        assert!(result.is_err());
    }

    #[test]
    fn splitting_empty_errors() {
        let mut container = dummy_empty();
        assert!(container.split(Axis::Vertical).is_err())
    }

    #[test]
    fn splitting_split_with_many_children_errors() {
        // This container could not exist because the bounds are wrong
        let mut container = dummy_split(
            Axis::default(),
            &[WindowId::from(1u32), WindowId::from(2u32)],
        );
        assert!(container.split(Axis::Vertical).is_err());
    }

    #[test]
    fn splitting_split_with_one_child_changes_axis() {
        use Axis::*;

        for (starting_axis, change_axis, exp_axis) in [
            (Horizontal, Vertical, Vertical),
            (Horizontal, Horizontal, Horizontal),
            (Vertical, Horizontal, Horizontal),
            (Vertical, Vertical, Vertical),
        ] {
            let mut container = dummy_split(starting_axis, &[WindowId::from(1u32)]);
            container.split(change_axis).unwrap();
            assert!(
                matches!(container, Container::Split { axis, children, .. } if axis == exp_axis && children.len() == 1)
            );
        }
    }

    #[test]
    fn splitting_leaf_converts_to_split_with_same_bounds() {
        let mut container = dummy_leaf(WindowId::from(1u32));

        // The same as above, but we can't clone it since containers contain a
        // Vec
        let leaf = dummy_leaf(WindowId::from(1u32));

        container.split(Axis::Vertical).unwrap();

        assert!(matches!(container, Container::Split{
            bounds, children, axis, padding
        } if bounds == dummy_bounds()
            && children == vec![leaf]
            && axis == Axis::Vertical
            && padding == 0.0
        ));
    }

    #[test]
    fn find_window_empty_is_none() {
        let container = dummy_empty();
        assert!(container.find_window(WindowId::from(0u32)).is_none());
    }

    #[test]
    fn find_window_leaf() {
        let target = WindowId::from(1u32);
        let container = dummy_leaf(target);

        assert!(container.find_window(target).is_some());
        assert!(container.find_window(WindowId::from(18u32)).is_none());
    }

    #[test]
    fn find_window_split() {
        let target = WindowId::from(1u32);
        let container = dummy_split(Axis::default(), &[target]);

        assert!(container.find_window(target).is_some());
        assert!(container.find_window(WindowId::from(18u32)).is_none());
    }

    #[test]
    fn window_ids_empty() {
        let container = dummy_empty();
        assert!(container.window_ids().is_empty());
    }

    #[test]
    fn window_ids_leaf() {
        let container = dummy_leaf(WindowId::from(1u32));
        assert_eq!(
            container.window_ids(),
            HashSet::from([WindowId::from(1u32)])
        );
    }

    #[test]
    fn window_ids_split() {
        let window_ids = [WindowId::from(1u32), WindowId::from(2u32)];
        let container = dummy_split(Axis::default(), &window_ids);
        assert_eq!(container.window_ids(), HashSet::from(window_ids));
    }

    #[test]
    fn remove_window_from_leaf_non_existent() {
        let mut container = dummy_leaf(WindowId::from(1u32));
        let target = WindowId::from(2u32);
        assert!(container.remove_window_from_leaf(target).unwrap().is_none());
    }

    #[test]
    fn remove_window_from_leaf_existent() {
        let mut container = dummy_leaf(WindowId::from(1u32));
        let target = WindowId::from(1u32);
        assert!(
            container
                .remove_window_from_leaf(target)
                .unwrap()
                .is_some_and(|id| id == target)
        );
    }

    #[test]
    fn is_parent_leaf_non_leaf() {
        let empty = dummy_empty();
        let split = dummy_split(
            Axis::default(),
            &[WindowId::from(1u32), WindowId::from(2u32)],
        );

        assert!(!empty.is_parent_leaf(WindowId::from(1u32)));
        assert!(!split.is_parent_leaf(WindowId::from(2u32)));
    }

    #[test]
    fn is_parent_leaf_leaf() {
        let target = WindowId::from(1u32);
        let parent = dummy_leaf(target);
        let non_parent = dummy_leaf(WindowId::from(2u32));

        assert!(parent.is_parent_leaf(target));
        assert!(!non_parent.is_parent_leaf(target));
    }

    #[test]
    fn remove_window_empty() {
        assert!(
            dummy_empty()
                .remove_window(WindowId::from(1u32), 0.0)
                .unwrap()
                .is_none()
        )
    }

    #[test]
    fn remove_window_leaf_target_exists() {
        let target = WindowId::from(1u32);
        let mut leaf = dummy_leaf(target);

        let res = leaf.remove_window(target, 0.0).unwrap();

        assert!(res.is_some_and(|id| id == target));
        assert!(matches!(leaf, Container::Empty { .. }))
    }

    #[test]
    fn remove_window_leaf_target_does_not_exist() {
        let target = WindowId::from(1u32);
        let mut leaf = dummy_leaf(WindowId::from(2u32));

        let res = leaf.remove_window(target, 0.0).unwrap();

        assert!(res.is_none());
        assert!(matches!(leaf, Container::Leaf { window,.. } if window.id == WindowId::from(2u32)));
    }

    #[test]
    fn remove_window_split_target_does_not_exist() {
        let target = WindowId::from(1u32);
        let mut split = dummy_split(Axis::default(), &[WindowId::from(2u32)]);

        let res = split.remove_window(target, 0.0).unwrap();
        assert!(res.is_none())
    }

    #[test]
    fn remove_window_split_target_exists_only_child() {
        let target = WindowId::from(1u32);
        let mut split = dummy_split(Axis::default(), &[target]);

        let res = split.remove_window(target, 0.0).unwrap();

        assert!(res.is_some_and(|id| id == target));
        assert!(matches!(split, Container::Empty { .. }))
    }

    #[test]
    fn remove_window_split_target_exists_two_children() {
        let target = WindowId::from(1u32);
        let mut split = dummy_split(
            Axis::default(),
            &[WindowId::from(1u32), WindowId::from(2u32)],
        );

        let res = split.remove_window(target, 0.0).unwrap();

        assert!(res.is_some_and(|id| id == target));
        assert!(matches!(split, Container::Split { children, .. }
            if children == vec![Container::Leaf {
                bounds: split.get_bounds(),
                padding: 0.0,
                window: dummy_window(WindowId::from(2u32)),
            }]
        ))
    }

    #[test]
    fn remove_window_split_target_exists_many_children() {
        let target = WindowId::from(1u32);
        let mut split = dummy_split(
            Axis::default(),
            &[
                WindowId::from(1u32),
                WindowId::from(2u32),
                WindowId::from(3u32),
            ],
        );

        let res = split.remove_window(target, 0.0).unwrap();

        let exp_child_bounds =
            spread_bounds_along_axis(split.get_bounds(), Axis::default(), 2, 0.0);

        assert!(res.is_some_and(|id| id == target));
        assert!(matches!(split, Container::Split { children, .. }
            if children == vec![
                Container::Leaf {
                    bounds: exp_child_bounds[0],
                    padding: 0.0,
                    window:dummy_window(WindowId::from(2u32))
                },
                Container::Leaf {
                    bounds:exp_child_bounds[1],
                    padding: 0.0,
                    window:dummy_window(WindowId::from(3u32))
                }
            ]
        ))
    }

    #[test]
    fn get_leaf_of_window_mut_empty() {
        let mut container = dummy_empty();
        let target = WindowId::from(1u32);

        assert!(container.parent_leaf_of_window_mut(target).is_none());
    }

    #[test]
    fn get_leaf_of_window_mut_leaf() {
        let target = WindowId::from(1u32);
        let mut is_parent = dummy_leaf(target);
        let mut not_parent = dummy_leaf(WindowId::from(2u32));

        assert!(is_parent.parent_leaf_of_window_mut(target).is_some_and(
            |leaf| matches!(leaf, Container::Leaf { window,.. } if window.id == target)
        ));

        assert!(not_parent.parent_leaf_of_window_mut(target).is_none());
    }

    #[test]
    fn get_leaf_of_window_split() {
        let target = WindowId::from(1u32);
        let mut exists = dummy_split(Axis::default(), &[WindowId::from(2u32), target]);
        let mut doest_not_exist = dummy_split(
            Axis::default(),
            &[WindowId::from(2u32), WindowId::from(3u32)],
        );

        assert!(exists.parent_leaf_of_window_mut(target).is_some());
        assert!(doest_not_exist.parent_leaf_of_window_mut(target).is_none());
    }

    #[test]
    fn spread_bounds_along_axis_horizontal() {
        let original = dummy_bounds();

        for &padding in &[0.0, 5.0, 10.0, 17.5] {
            for n in 1usize..=8 {
                let out = spread_bounds_along_axis(original, Axis::Horizontal, n, padding);
                assert_eq!(out.len(), n);

                let total_inner_gap = (n - 1) as f64 * padding;
                let available_width = original.width - 2.0 * padding - total_inner_gap;
                let child_width = available_width / n as f64;
                let expected_height = original.height - 2.0 * padding;

                // The first child starts after the correct padding
                assert!(approx(out[0].x, original.x + padding));
                assert!(approx(out[0].y, original.y + padding));

                // The last child has the correct amount of padding after it
                let last = &out[n - 1];
                assert!(approx(
                    last.x + last.width,
                    original.x + original.width - padding
                ));

                // Each inner child's bounds are correct
                for (i, b) in out.iter().enumerate() {
                    assert!(approx(b.width, child_width));
                    assert!(approx(b.height, expected_height));
                    assert!(approx(b.y, original.y + padding));

                    let expected_x = original.x + padding + i as f64 * (child_width + padding);
                    assert!(approx(b.x, expected_x));
                }

                // The difference between child positions is the padding
                for i in 1..n {
                    let prev = &out[i - 1];
                    let cur = &out[i];
                    assert!(approx(cur.x - (prev.x + prev.width), padding));
                }

                // Validate that the bounds aren't overlapping (bar some margin
                // for error)
                for i in 1..n {
                    assert!(out[i].x >= out[i - 1].x + out[i - 1].width - EPSILON);
                }

                // Full available width is spanned
                let covered: f64 = out.iter().map(|b| b.width).sum();
                assert!(approx(covered, available_width));
            }
        }
    }

    #[test]
    fn spread_bounds_along_axis_vertical() {
        let original = dummy_bounds();

        for &padding in &[0.0, 3.0, 8.0, 20.0] {
            for n in 1usize..=8 {
                let out = spread_bounds_along_axis(original, Axis::Vertical, n, padding);

                assert_eq!(out.len(), n);

                let total_inner_gap = (n - 1) as f64 * padding;
                let available_height = original.height - 2.0 * padding - total_inner_gap;
                let child_height = available_height / n as f64;
                let expected_width = original.width - 2.0 * padding;

                assert!(approx(out[0].y, original.y + padding));
                assert!(approx(out[0].x, original.x + padding));

                let last = &out[n - 1];
                assert!(approx(
                    last.y + last.height,
                    original.y + original.height - padding
                ));

                for (i, b) in out.iter().enumerate() {
                    assert!(approx(b.height, child_height));
                    assert!(approx(b.width, expected_width));
                    assert!(approx(b.x, original.x + padding));

                    let expected_y = original.y + padding + i as f64 * (child_height + padding);
                    assert!(approx(b.y, expected_y));
                }

                for i in 1..n {
                    let prev = &out[i - 1];
                    let cur = &out[i];
                    assert!(approx(cur.y - (prev.y + prev.height), padding));
                }

                for i in 1..n {
                    assert!(out[i].y >= out[i - 1].y + out[i - 1].height - EPSILON);
                }

                let covered: f64 = out.iter().map(|b| b.height).sum();
                assert!(approx(covered, available_height));
            }
        }
    }

    #[test]
    fn n_equals_one_fills_inner_area() {
        let original = dummy_bounds();

        for &padding in &[0.0, 5.0, 25.0] {
            for axis in [Axis::Horizontal, Axis::Vertical] {
                let out = spread_bounds_along_axis(original, axis, 1, padding);
                let b = &out[0];

                assert!(approx(b.x, original.x + padding));
                assert!(approx(b.y, original.y + padding));
                assert!(approx(b.width, original.width - 2.0 * padding));
                assert!(approx(b.height, original.height - 2.0 * padding));
            }
        }
    }

    #[test]
    fn spread_bounds_along_axis_symmetry() {
        let original = Bounds {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 800.0,
        };

        let padding = 12.0;
        let n = 4;

        let h = spread_bounds_along_axis(original, Axis::Horizontal, n, padding);

        let transposed = Bounds {
            x: original.x,
            y: original.y,
            width: original.height,
            height: original.width,
        };

        let v = spread_bounds_along_axis(transposed, Axis::Vertical, n, padding);

        for (bh, bv) in h.iter().zip(v.iter()) {
            assert!(approx(bh.width, bv.height));
            assert!(approx(bh.height, bv.width));
        }
    }

    fn two_window_split() -> (Container, WindowId, WindowId) {
        let mut root = Container::Empty {
            bounds: Bounds {
                x: 0.0,
                y: 0.0,
                width: 900.0,
                height: 600.0,
            },
        };
        let a = WindowId::from(1u32);
        let b = WindowId::from(2u32);
        root.add_window(
            Window {
                id: a,
                min_width: 0.0,
                min_height: 0.0,
            },
            0.0,
        )
        .unwrap();
        root.add_window(
            Window {
                id: b,
                min_width: 0.0,
                min_height: 0.0,
            },
            0.0,
        )
        .unwrap();
        (root, a, b)
    }

    /// After a nested split's only window is removed, the sibling of that nested
    /// split should expand to fill the whole parent — not keep its old half-width.
    ///
    /// Scenario
    /// --------
    ///   root (H-split, 900 wide)
    ///   ├── leaf A  (450 wide)          ← stays
    ///   └── inner (H-split, 450 wide)
    ///       └── leaf B  (450 wide)     ← removed; inner collapses to Empty
    ///
    /// After removal of B:
    ///   root should collapse to a single-child split (or Empty→leaf)
    ///   and A's bounds should span the full 900 width.
    #[test]
    fn remove_nested_window_rebalances_parent() {
        let (mut root, a, b) = two_window_split();

        // Split leaf B into its own inner split (simulates user pressing split).
        // parent_leaf_of_window_mut returns the Leaf itself; calling split() on it
        // converts it to a Split containing that leaf.
        root.parent_leaf_of_window_mut(b)
            .unwrap()
            .split(Axis::Horizontal)
            .unwrap();

        // Sanity: both windows still present.
        assert!(root.contains_window(a));
        assert!(root.contains_window(b));

        // Remove B — it now lives inside the inner split.
        let removed = root.remove_window(b, 0.0).unwrap();
        assert_eq!(removed, Some(b), "B must be reported as removed");
        assert!(!root.contains_window(b));

        // A must still exist.
        assert!(root.contains_window(a));

        // A's bounds must now span the full 900 px, not stay at the old 450.
        let bounds_map = root.window_bounds_by_id();
        let a_bounds = bounds_map[&a];
        assert!(
            approx(a_bounds.width, 900.0),
            "after nested removal, surviving window must expand to full width; got {}",
            a_bounds.width
        );
    }

    /// Simpler case: direct removal from a two-child split still works after the
    /// refactor (regression guard for the direct path).
    #[test]
    fn remove_direct_child_rebalances_sibling() {
        let (mut root, a, b) = two_window_split();

        root.remove_window(a, 0.0).unwrap();

        assert!(!root.contains_window(a));
        assert!(root.contains_window(b));

        let bounds_map = root.window_bounds_by_id();
        let b_bounds = bounds_map[&b];
        assert!(
            approx(b_bounds.width, 900.0),
            "surviving window must fill full width after direct removal; got {}",
            b_bounds.width
        );
    }

    /// Three-window split: removing the middle window rebalances the outer two to
    /// each take half the space (no ghost third slot).
    #[test]
    fn remove_middle_of_three_no_ghost_slot() {
        let mut root = Container::Empty {
            bounds: Bounds {
                x: 0.0,
                y: 0.0,
                width: 900.0,
                height: 600.0,
            },
        };
        let a = WindowId::from(1u32);
        let b = WindowId::from(2u32);
        let c = WindowId::from(3u32);
        for id in [a, b, c] {
            root.add_window(
                Window {
                    id,
                    min_width: 0.0,
                    min_height: 0.0,
                },
                0.0,
            )
            .unwrap();
        }

        root.remove_window(b, 0.0).unwrap();

        let bounds_map = root.window_bounds_by_id();
        assert!(approx(bounds_map[&a].width, 450.0));
        assert!(approx(bounds_map[&c].width, 450.0));
        // The two survivors must not overlap and must tile perfectly.
        assert!(approx(bounds_map[&a].x, 0.0));
        assert!(approx(bounds_map[&c].x, 450.0));
    }
}
