use crate::Error;
use ax_ui::{Callback, Observer};
use core_graphics::{Bounds, CGPoint, CGSize};
use std::{hash::Hash, rc::Rc};

#[derive(Debug)]
pub(crate) struct Window {
    cg: core_graphics::Window,
    ax: ax_ui::Window,
    lock_observer: Observer,
    lock_callback: Callback,
    bounds: Bounds,
}

impl Hash for Window {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cg.number().hash(state);
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        self.cg.number() == other.cg.number()
    }
}

impl Window {
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

    pub(crate) fn try_new(cg_window: core_graphics::Window, bounds: Bounds) -> crate::Result<Self> {
        let mut ax_window = ax_ui::Window::new(cg_window.owner_pid(), cg_window.number().into())
            .map_err(Error::AxUi)?;

        let lock_callback = Window::lock_callback(ax_window, bounds);
        let ax_window_ref = ax_window.window_ref();
        let observer =
            Observer::try_new(cg_window.owner_pid(), &lock_callback).map_err(Error::AxUi)?;

        observer
            .add_notification(
                ax_window_ref,
                ax_ui::Window::RESIZED_ATTR,
                lock_callback.ctx,
            )
            .map_err(Error::AxUi)?;
        observer
            .add_notification(ax_window_ref, ax_ui::Window::MOVED_ATTR, lock_callback.ctx)
            .map_err(Error::AxUi)?;

        observer.run();

        Ok(Self {
            cg: cg_window,
            ax: ax_window,
            lock_observer: observer,
            lock_callback,
            bounds,
        })
    }

    fn lock_callback(ax: ax_ui::Window, bounds: Bounds) -> Callback {
        let context = LockContext {
            window: Rc::new(ax),
            point: bounds.point(),
            size: bounds.size(),
        };

        Callback::new(context, |ctx| {
            // TODO: logging
            let _ = ctx.window.resize(ctx.size.width, ctx.size.height);
            let _ = ctx.window.move_to(ctx.point.x, ctx.point.y);
        })
    }
}

struct LockContext {
    window: Rc<ax_ui::Window>,
    point: CGPoint,
    size: CGSize,
}
