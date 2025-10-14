use crate::Error;
use ax_ui::{Callback, Observer};
use core_graphics::{Bounds, CGPoint, CGSize};
use std::rc::Rc;

#[derive(Debug, Clone)]
pub(crate) struct Window<'a> {
    cg: &'a core_graphics::Window,
    ax: ax_ui::Window,
    lock_observer: Observer,
    bounds: Bounds,
}

impl<'a> Window<'a> {
    pub(crate) fn ax(&self) -> &ax_ui::Window {
        &self.ax
    }

    pub(crate) fn cg(&self) -> &core_graphics::Window {
        &self.cg
    }

    pub(crate) fn init(&mut self) -> crate::Result<()> {
        self.ax
            .move_to(self.bounds.x, self.bounds.y)
            .map_err(Error::AxUi)?;
        self.ax
            .resize(self.bounds.width, self.bounds.height)
            .map_err(Error::AxUi)
    }

    pub(crate) fn try_new(
        cg_window: &'a core_graphics::Window,
        bounds: Bounds,
    ) -> crate::Result<Self> {
        let search_name = cg_window.name().unwrap().to_string();
        let mut ax_window =
            ax_ui::Window::new(cg_window.owner_pid(), search_name).map_err(Error::AxUi)?;

        let lock_callback = Window::lock_callback(&ax_window, bounds);

        let observer =
            Observer::try_new(cg_window.owner_pid(), &lock_callback).map_err(Error::AxUi)?;
        observer
            .add_notification(ax_window.window_ref(), "AXResized", lock_callback.ctx)
            .map_err(Error::AxUi)?;
        observer
            .add_notification(ax_window.window_ref(), "AXMoved", lock_callback.ctx)
            .map_err(Error::AxUi)?;

        observer.run();

        Ok(Self {
            cg: cg_window,
            ax: ax_window,
            lock_observer: observer,
            bounds,
        })
    }

    fn lock_callback(ax: &ax_ui::Window, bounds: Bounds) -> Rc<Callback> {
        let context = LockContext {
            window: Rc::new(ax.clone()),
            point: bounds.point(),
            size: bounds.size(),
        };

        Rc::new(Callback::new(context, |ctx| {
            let _ = ctx.window.resize(ctx.size.width, ctx.size.height);
            let _ = ctx.window.move_to(ctx.point.x, ctx.point.y);
        }))
    }
}

struct LockContext {
    window: Rc<ax_ui::Window>,
    point: CGPoint,
    size: CGSize,
}
