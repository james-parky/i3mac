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
    pub fn disable_observers(&mut self) -> crate::Result<()> {
        let _ = self
            .lock_observer
            .remove_notification(self.ax.window_ref(), "AXResized")
            .map_err(Error::AxUi);
        let _ = self
            .lock_observer
            .remove_notification(self.ax.window_ref(), "AXMoved")
            .map_err(Error::AxUi);

        Ok(())
    }

    pub fn enable_observers(&mut self) -> crate::Result<()> {
        self.lock_observer
            .add_notification(self.ax.window_ref(), "AXResized", self.lock_callback.ctx)
            .map_err(Error::AxUi)?;
        self.lock_observer
            .add_notification(self.ax.window_ref(), "AXMoved", self.lock_callback.ctx)
            .map_err(Error::AxUi)
    }

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
            .map_err(Error::AxUi)?;

        self.lock_observer
            .add_notification(
                self.ax.window_ref(),
                ax_ui::Window::RESIZED_ATTR,
                self.lock_callback.ctx,
            )
            .map_err(Error::AxUi)?;
        self.lock_observer
            .add_notification(
                self.ax.window_ref(),
                ax_ui::Window::MOVED_ATTR,
                self.lock_callback.ctx,
            )
            .map_err(Error::AxUi)
    }

    pub(crate) fn try_new(cg_window: core_graphics::Window, bounds: Bounds) -> crate::Result<Self> {
        let mut ax_window = ax_ui::Window::new(cg_window.owner_pid(), cg_window.number().into())
            .map_err(Error::AxUi)?;

        let lock_callback = Window::lock_callback(ax_window.clone(), bounds);
        let observer =
            Observer::try_new(cg_window.owner_pid(), &lock_callback).map_err(Error::AxUi)?;

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

    pub fn update_bounds_no_observer_update(&mut self, new_bounds: Bounds) -> crate::Result<()> {
        self.bounds = new_bounds;

        // Just update the callback context, don't touch observers
        self.lock_callback = Self::lock_callback(self.ax().clone(), new_bounds);

        self.ax
            .move_to(new_bounds.x, new_bounds.y)
            .map_err(Error::AxUi)?;
        self.ax
            .resize(new_bounds.width, new_bounds.height)
            .map_err(Error::AxUi)?;

        Ok(())
    }

    pub fn update_bounds(&mut self, new_bounds: Bounds) -> crate::Result<()> {
        self.bounds = new_bounds;

        let _ = self
            .lock_observer
            .remove_notification(self.ax.window_ref(), "AXResized")
            .map_err(Error::AxUi);

        let _ = self
            .lock_observer
            .remove_notification(self.ax.window_ref(), "AXMoved")
            .map_err(Error::AxUi);

        let new_callback = Self::lock_callback(self.ax().clone(), new_bounds);
        let old_callback = std::mem::replace(&mut self.lock_callback, new_callback);
        drop(old_callback);

        self.lock_observer
            .add_notification(self.ax.window_ref(), "AXResized", self.lock_callback.ctx)
            .map_err(Error::AxUi)?;

        self.lock_observer
            .add_notification(self.ax.window_ref(), "AXMoved", self.lock_callback.ctx)
            .map_err(Error::AxUi)?;

        self.ax
            .move_to(new_bounds.x, new_bounds.y)
            .map_err(Error::AxUi)?;
        self.ax
            .resize(new_bounds.width, new_bounds.height)
            .map_err(Error::AxUi)?;

        Ok(())
    }
}

struct LockContext {
    window: Rc<ax_ui::Window>,
    point: CGPoint,
    size: CGSize,
}
