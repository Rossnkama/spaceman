use super::*;

const GAPS: &str = "version = 1\n[gaps]\ninner.horizontal = 10\ninner.vertical = 6\n\
                    outer.left = 20\nouter.bottom = 30\nouter.top = 40\nouter.right = 50\n";

fn tiled(
    config: &str,
    windows: &[(crate::PhysicalWindowId, i32)],
) -> WorkspaceRuntime<FakeBackend> {
    let mut backend = FakeBackend::default();
    for (window, minimum) in windows {
        backend.frames.insert(window.clone(), frame(0));
        if *minimum > 0 {
            backend.minimum_widths.insert(window.clone(), *minimum);
        }
    }
    let mut runtime = runtime_with_config(backend, Config::parse(config).unwrap());
    for (window, _) in windows {
        bind(&mut runtime, WorkspaceId::ONE, window);
    }
    runtime
}

fn layout(runtime: &mut WorkspaceRuntime<FakeBackend>) {
    runtime.layout_workspace(
        WorkspaceId::ONE,
        outputs().main().frame,
        &AtomicBool::new(false),
    );
}

fn rect(x: i32, y: i32, width: i32, height: i32) -> crate::Rect {
    crate::Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn tiles_keep_the_outer_gaps_at_the_edges_and_the_inner_gap_between_them() {
    let (left, top, bottom) = (window(10), window(11), window(12));
    let mut runtime = tiled(
        GAPS,
        &[(left.clone(), 0), (top.clone(), 0), (bottom.clone(), 0)],
    );
    assert!(
        runtime
            .workspaces
            .engine_mut(WorkspaceId::ONE)
            .move_window_relative(&bottom, &top, Direction::Down)
    );

    layout(&mut runtime);

    // The 1000x800 display leaves x 20..950 and y 40..770 inside the outer gaps.
    assert_eq!(runtime.backend.frames[&left], rect(20, 40, 460, 730));
    assert_eq!(runtime.backend.frames[&top], rect(490, 40, 460, 362));
    assert_eq!(runtime.backend.frames[&bottom], rect(490, 408, 460, 362));
}

#[test]
fn a_lone_tile_and_fullscreen_layout_fill_the_display_inside_the_outer_gaps() {
    let (first, second) = (window(10), window(11));
    let mut lone = tiled(GAPS, &[(first.clone(), 0)]);
    layout(&mut lone);
    assert_eq!(lone.backend.frames[&first], rect(20, 40, 930, 730));

    let mut runtime = tiled(GAPS, &[(first.clone(), 0), (second.clone(), 0)]);
    runtime.backend.focused = Some(first.clone());
    runtime
        .toggle_fullscreen_layout(outputs(), &AtomicBool::new(false))
        .unwrap();
    assert_eq!(runtime.backend.frames[&first], rect(20, 40, 930, 730));
    assert_eq!(runtime.backend.frames[&second], rect(20, 40, 930, 730));
}

#[test]
fn pointer_and_keyboard_resizes_keep_the_inner_gap() {
    let (first, second) = (window(10), window(11));
    let mut runtime = tiled(GAPS, &[(first.clone(), 0), (second.clone(), 0)]);
    runtime.backend.focused = Some(first.clone());
    layout(&mut runtime);
    let cancelled = AtomicBool::new(false);

    // Drag the first tile's right edge from x 480 to x 600. The dragged window is the
    // user's to move, so only its neighbor is written.
    runtime
        .resize_window(
            first.clone(),
            rect(20, 40, 580, 730),
            false,
            outputs(),
            &cancelled,
        )
        .unwrap();
    assert_eq!(runtime.backend.frames[&second], rect(610, 40, 340, 730));

    runtime
        .resize_focused(ResizeDirection::Grow, outputs(), &cancelled)
        .unwrap();
    assert_eq!(runtime.backend.frames[&first], rect(20, 40, 630, 730));
    assert_eq!(runtime.backend.frames[&second], rect(660, 40, 290, 730));
}

#[test]
fn a_window_that_refuses_its_tile_width_still_gets_its_minimum_between_gaps() {
    let (left, right) = (window(10), window(11));
    let mut runtime = tiled(GAPS, &[(left.clone(), 640), (right.clone(), 0)]);

    layout(&mut runtime);

    assert_eq!(runtime.backend.frames[&left], rect(20, 40, 640, 730));
    assert_eq!(runtime.backend.frames[&right], rect(670, 40, 280, 730));
}

#[test]
fn a_reloaded_config_retiles_with_the_new_gaps() {
    let (first, second) = (window(10), window(11));
    let mut runtime = tiled("version = 1", &[(first.clone(), 0), (second.clone(), 0)]);
    layout(&mut runtime);
    assert_eq!(runtime.backend.frames[&first], rect(0, 0, 500, 800));
    assert_eq!(runtime.backend.frames[&second], rect(500, 0, 500, 800));

    runtime.replace_config(Config::parse(GAPS).unwrap());
    layout(&mut runtime);

    assert_eq!(runtime.backend.frames[&first], rect(20, 40, 460, 730));
    assert_eq!(runtime.backend.frames[&second], rect(490, 40, 460, 730));
}
