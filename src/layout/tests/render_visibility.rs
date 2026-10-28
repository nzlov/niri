use super::*;
use crate::backend::Headless;
use crate::render_helpers::RenderTarget;

fn set_up(options: Options) -> (Layout<TestWindow>, Vec<TestWindow>, Headless) {
    let mut layout = Layout::with_options(Clock::with_time(Duration::ZERO), options);
    Op::AddOutput(1).apply(&mut layout);
    for id in 1..=6 {
        let mut params = TestWindowParams::new(id);
        params.bbox = Rectangle::from_size(Size::from((1000, 200)));
        Op::AddWindow { params }.apply(&mut layout);
    }
    Op::CompleteAnimations.apply(&mut layout);

    let mut windows = layout
        .windows()
        .map(|(_, win)| win.clone())
        .collect::<Vec<_>>();
    windows.sort_by_key(|win| *win.id());
    for win in &windows {
        win.0
            .render_bounds
            .set(Some(Rectangle::from_size(win.size())));
    }

    // Ensure the test actually puts the first window outside the scrolling viewport.
    let ws = layout.active_workspace().unwrap();
    let (tile, pos, _) = ws
        .tiles_with_render_positions()
        .find(|(tile, _, _)| *tile.window().id() == 1)
        .unwrap();
    assert!(pos.x + tile.tile_size().w < -100.);

    let mut backend = Headless::new();
    backend.add_renderer().unwrap();
    (layout, windows, backend)
}

fn render(layout: &mut Layout<TestWindow>, backend: &mut Headless, target: RenderTarget) {
    for (_, win) in layout.windows() {
        win.0.render_count.set(0);
        win.0.offscreen_reset_count.set(0);
    }
    layout.update_render_elements(None);
    backend
        .with_primary_renderer(|renderer| {
            let ctx = RenderCtx {
                renderer,
                target,
                xray: None,
            };
            layout
                .monitors()
                .next()
                .unwrap()
                .render_workspaces(ctx, true, &mut |_| {});
        })
        .unwrap();
}

#[test]
fn egl_offscreen_static_windows_skip_render_construction() {
    let (mut layout, windows, mut backend) = set_up(Options::default());
    let first = &windows[0];
    let last = windows.last().unwrap();

    render(&mut layout, &mut backend, RenderTarget::Output);
    assert_eq!(first.0.render_count.get(), 0);
    assert_eq!(first.0.offscreen_reset_count.get(), 1);
    assert_eq!(last.0.render_count.get(), 1);

    for target in [RenderTarget::ScreenCapture, RenderTarget::Screencast] {
        render(&mut layout, &mut backend, target);
        assert_eq!(first.0.render_count.get(), 1);
    }

    // Unknown surface bounds must keep the old render path.
    first.0.render_bounds.set(None);
    render(&mut layout, &mut backend, RenderTarget::Output);
    assert_eq!(first.0.render_count.get(), 1);

    // CSD, subsurfaces or popups may extend far beyond the visual window geometry.
    first
        .0
        .render_bounds
        .set(Some(Rectangle::new((0, 0).into(), (10000, 200).into())));
    render(&mut layout, &mut backend, RenderTarget::Output);
    assert_eq!(first.0.render_count.get(), 1);
}

#[test]
fn egl_offscreen_shadow_extending_into_view_keeps_rendering() {
    let mut options = Options::default();
    options.layout.shadow.on = true;
    options.layout.shadow.softness = 10000.;
    let (mut layout, windows, mut backend) = set_up(options);

    render(&mut layout, &mut backend, RenderTarget::Output);
    assert_eq!(windows[0].0.render_count.get(), 1);
}

#[test]
fn egl_offscreen_animation_and_unit_zoom_overview_keep_rendering() {
    let mut options = Options::default();
    options.overview.zoom = 1.;
    let (mut layout, windows, mut backend) = set_up(options);
    let first = &windows[0];

    layout.start_open_animation_for_window(first.id());
    render(&mut layout, &mut backend, RenderTarget::Output);
    assert!(first.0.render_count.get() > 0);
    Op::CompleteAnimations.apply(&mut layout);

    layout.toggle_overview();
    Op::CompleteAnimations.apply(&mut layout);
    render(&mut layout, &mut backend, RenderTarget::Output);
    assert_eq!(first.0.render_count.get(), 1);
}
