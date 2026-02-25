// Tests for src/display/mod.rs, physical.rs, and logical.rs
//
// Drop this into src/display/tests.rs and add `mod tests;` to src/display/mod.rs,
// or inline each block into the relevant file's existing `#[cfg(test)]` section.
//
// The tests are grouped by the layer they exercise:
//   1. PhysicalDisplay
//   2. Displays (the top-level manager)
//
// They deliberately exercise the two confirmed bugs so you can watch them
// fail before applying the fixes, then pass after.

#[cfg(test)]
mod display_tests {
    use crate::container::{Axis, Window};
    use crate::display::physical::{Config, PhysicalDisplay};
    use crate::display::{AddWindowResult, Displays, LogicalDisplayId, PhysicalDisplayId};
    use core_graphics::{Bounds, WindowId};

    // ── helpers ──────────────────────────────────────────────────────────────

    fn bounds() -> Bounds {
        Bounds {
            x: 0.0,
            y: 0.0,
            width: 2560.0,
            height: 1440.0,
        }
    }

    fn small_bounds() -> Bounds {
        // Deliberately narrow so that many windows cannot fit side-by-side,
        // making CannotFitWindow fire and forcing a new logical display.
        Bounds {
            x: 0.0,
            y: 0.0,
            width: 300.0,
            height: 300.0,
        }
    }

    fn cfg() -> Config {
        Config {
            window_padding: None,
        }
    }

    fn wid(n: u32) -> WindowId {
        WindowId::from(n)
    }

    fn win(id: u32) -> Window {
        Window {
            id: wid(id),
            min_width: 0.0,
            min_height: 0.0,
        }
    }

    /// A window with a large minimum width so it cannot share an LD with others.
    fn wide_win(id: u32) -> Window {
        Window {
            id: wid(id),
            min_width: 250.0,
            min_height: 0.0,
        }
    }

    fn pid(n: usize) -> PhysicalDisplayId {
        PhysicalDisplayId(n)
    }

    fn lid(n: usize) -> LogicalDisplayId {
        LogicalDisplayId(n)
    }

    // ── PhysicalDisplay ───────────────────────────────────────────────────────

    /// Adding a window to the active LD works.
    #[test]
    fn pd_add_window_appears_in_active_ld() {
        let mut pd = PhysicalDisplay::new(lid(0), bounds(), cfg());
        pd.add_window(win(1)).unwrap();

        let ids = pd.active_logical_display().unwrap().window_ids();
        assert!(ids.contains(&wid(1)));
    }

    /// Removing a window that lives on the *active* LD succeeds.
    #[test]
    fn pd_remove_window_on_active_ld() {
        let mut pd = PhysicalDisplay::new(lid(0), bounds(), cfg());
        pd.add_window(win(1)).unwrap();

        let result = pd.remove_window(wid(1)).unwrap();
        assert_eq!(result, Some(wid(1)));
        assert!(pd.window_ids().is_empty());
    }

    /// BUG #2 regression: removing a window that lives on a *non-active* LD
    /// must still find and remove it, not silently return None.
    ///
    /// Without the fix this test fails: remove_window searches only the active
    /// LD, finds nothing, and returns Ok(None) while the window stays orphaned.
    #[test]
    fn pd_remove_window_on_non_active_ld() {
        let mut pd = PhysicalDisplay::new(lid(0), bounds(), cfg());
        pd.add_window(win(1)).unwrap();

        // Create a second LD and switch to it.
        pd.create_logical_display(lid(1));
        pd.switch_to(lid(1)).unwrap();

        // Window 1 is now on LD0 which is inactive.
        let result = pd.remove_window(wid(1));
        assert!(
            result.is_ok() && result.unwrap() == Some(wid(1)),
            "remove_window must find the window even on a non-active LD"
        );
        assert!(!pd.window_ids().contains(&wid(1)));
    }

    /// Switching to an LD that has no windows removes the old (now-empty) LD.
    #[test]
    fn pd_switch_to_removes_empty_source_ld() {
        let mut pd = PhysicalDisplay::new(lid(0), bounds(), cfg());
        // LD0 has no windows.
        pd.create_logical_display(lid(1));

        let removed = pd.switch_to(lid(1)).unwrap();
        assert!(removed, "empty LD0 should have been cleaned up");
        assert!(!pd.has_logical_display(lid(0)));
    }

    /// Switching to an LD whose source has windows does NOT remove the source.
    #[test]
    fn pd_switch_to_keeps_non_empty_source_ld() {
        let mut pd = PhysicalDisplay::new(lid(0), bounds(), cfg());
        pd.add_window(win(1)).unwrap();
        pd.create_logical_display(lid(1));

        let removed = pd.switch_to(lid(1)).unwrap();
        assert!(!removed, "LD0 still has windows, must not be removed");
        assert!(pd.has_logical_display(lid(0)));
    }

    /// Switching to the already-active LD is a no-op.
    #[test]
    fn pd_switch_to_same_ld_is_noop() {
        let mut pd = PhysicalDisplay::new(lid(0), bounds(), cfg());
        let changed = pd.switch_to(lid(0)).unwrap();
        assert!(!changed);
    }

    // ── Displays (top-level manager) ──────────────────────────────────────────

    /// add_physical registers one logical display and sets it as active.
    #[test]
    fn displays_add_physical_registers_logical() {
        let mut d = Displays::default();
        d.add_physical(pid(0), bounds(), cfg());

        let lids = d.logical_ids(pid(0));
        assert_eq!(lids.len(), 1);
        assert_eq!(d.active_logical_display_id(), lid(0));
    }

    /// Two physical displays get distinct logical IDs.
    #[test]
    fn displays_two_physical_displays_distinct_logical_ids() {
        let mut d = Displays::default();
        d.add_physical(pid(0), bounds(), cfg());
        d.add_physical(pid(1), bounds(), cfg());

        let lids0 = d.logical_ids(pid(0));
        let lids1 = d.logical_ids(pid(1));
        let overlap: Vec<_> = lids0.intersection(&lids1).collect();
        assert!(
            overlap.is_empty(),
            "physical displays must not share logical IDs"
        );
    }

    /// Adding a window goes to the currently focused logical display.
    #[test]
    fn displays_add_window_lands_on_active_ld() {
        let mut d = Displays::default();
        d.add_physical(pid(0), bounds(), cfg());

        let target_lid = d.active_logical_display_id();
        let assigned_lid = d.add_window(win(1)).unwrap();

        assert_eq!(assigned_lid, target_lid);
    }

    /// When a window cannot fit on the active LD, a new LD is created for it.
    #[test]
    fn displays_add_window_overflows_to_new_ld() {
        let mut d = Displays::default();
        // Use small bounds so the second wide window cannot fit alongside the first.
        d.add_physical(pid(0), small_bounds(), cfg());

        let lid0 = d.add_window(wide_win(1)).unwrap();
        let lid1 = d.add_window(wide_win(2)).unwrap();

        assert!(matches!(lid0, AddWindowResult::Active(_)));
        assert!(matches!(lid1, AddWindowResult::Overflow(_)));
        assert_eq!(d.logical_ids(pid(0)).len(), 2);
    }

    /// remove_window via the Displays manager finds the window regardless of
    /// which LD it lives on (delegates the fix down to PhysicalDisplay).
    #[test]
    fn displays_remove_window_on_non_active_ld() {
        let mut d = Displays::default();
        d.add_physical(pid(0), bounds(), cfg());

        // Add a window, then move focus away by switching LD.
        d.add_window(win(1)).unwrap();

        // The window's PD
        let owning_pid = d.display_of_window(wid(1)).unwrap();

        // Create a new LD through the manager (keeps active_logical_display_ids
        // in sync) then switch focus to it so win(1) is on the inactive LD.
        let new_lid = d.next_logical_display_id(pid(0)).unwrap();
        {
            let pd = d.active_physical_display_mut();
            pd.create_logical_display(new_lid);
            pd.switch_to(new_lid).unwrap();
        }

        // Now remove the window — it lives on the non-active LD.
        let result = d.remove_window(owning_pid, wid(1));
        assert!(
            result.is_ok(),
            "remove_window must not panic or error for a non-active LD window"
        );
        assert_eq!(result.unwrap(), Some(wid(1)));
    }

    /// The move-window cycle that caused the original crash:
    /// move to new LD → move back → move again must not panic.
    ///
    /// "Move" here is: remove from current LD, add_window_to_logical on target.
    #[test]
    fn displays_repeated_move_window_does_not_panic() {
        let mut d = Displays::default();
        d.add_physical(pid(0), bounds(), cfg());

        // Ensure both LD0 and LD1 exist.
        d.add_window(win(1)).unwrap(); // lands on LD0

        // Create LD1 through the manager so active_logical_display_ids stays
        // in sync. The bug was that calling pd.create_logical_display() directly
        // bypasses this map, causing add_window_to_logical to panic.
        let new_lid = d.create_logical_display(pid(0));
        // new_lid should be lid(1) since lid(0) is already taken.
        assert_eq!(new_lid, lid(1));

        let do_move = |d: &mut Displays,
                       from_lid: LogicalDisplayId,
                       to_lid: LogicalDisplayId,
                       window: u32| {
            let owning_pid = d.display_of_window(wid(window)).unwrap();
            d.remove_window(owning_pid, wid(window))
                .expect("remove must succeed");
            d.add_window_to_logical(win(window), to_lid)
                .expect("add must succeed");
        };

        // Move 1 → LD1
        do_move(&mut d, lid(0), lid(1), 1);
        // Move 1 → back to LD0
        do_move(&mut d, lid(1), lid(0), 1);
        // Move 1 → LD1 again (this is where the crash happened)
        do_move(&mut d, lid(0), lid(1), 1);

        // Window must be tracked correctly after all moves.
        assert!(d.display_of_window(wid(1)).is_some());
    }

    /// display_of_window returns None for an unknown window.
    #[test]
    fn displays_display_of_window_unknown() {
        let mut d = Displays::default();
        d.add_physical(pid(0), bounds(), cfg());
        assert!(d.display_of_window(wid(99)).is_none());
    }

    /// display_of_window returns the correct PD after a window is added.
    #[test]
    fn displays_display_of_window_known() {
        let mut d = Displays::default();
        d.add_physical(pid(0), bounds(), cfg());
        d.add_window(win(42)).unwrap();
        assert_eq!(d.display_of_window(wid(42)), Some(pid(0)));
    }

    /// focus_display switches the active physical display.
    #[test]
    fn displays_focus_display_switches_active() {
        let mut d = Displays::default();
        d.add_physical(pid(0), bounds(), cfg());
        d.add_physical(pid(1), bounds(), cfg());

        // After adding pid(1) the active PD is pid(1); lid(1) is its logical ID.
        let lid_on_pd1 = d.active_logical_display_id();

        // Focus back to the logical display that lives on pid(0).
        let lid_on_pd0 = *d.logical_ids(pid(0)).iter().next().unwrap();
        d.focus_display(lid_on_pd0);

        assert_eq!(d.active_logical_display_id(), lid_on_pd0);
        assert_ne!(d.active_logical_display_id(), lid_on_pd1);
    }
}
