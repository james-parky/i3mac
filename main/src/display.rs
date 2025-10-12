use crate::window::Window;
use core_graphics::Bounds;

#[derive(Debug, Default)]
enum Direction {
    Vertical,
    #[default]
    Horizontal,
}

#[derive(Debug)]
enum Container<'a> {
    Leaf(Window<'a>),
    Split {
        direction: Direction,
        children: Vec<Container<'a>>,
    },
}

impl<'a> Container<'a> {
    fn try_from_window(window: &'a core_graphics::Window, bounds: Bounds) -> crate::Result<Self> {
        let mut w = Window::try_new(window, bounds)?;
        w.init()?;
        Ok(Self::Leaf(w))
    }

    fn try_from_windows(
        windows: &'a [core_graphics::Window],
        bounds: &[Bounds],
    ) -> crate::Result<Self> {
        let children: Vec<Container> = windows
            .iter()
            .zip(bounds.iter())
            .map(|(w, &b)| {
                let mut w = Window::try_new(&w, b)?;
                w.init()?;
                Ok(Container::Leaf(w))
            })
            .collect::<crate::Result<_>>()?;

        Ok(Self::Split {
            direction: Direction::default(),
            children,
        })
    }
}

#[derive(Debug)]
pub(crate) struct Display<'a> {
    id: u64,
    bounds: Bounds,
    root: Container<'a>,
}

impl<'a> Display<'a> {
    pub(crate) fn try_new(display: &'a core_graphics::Display) -> crate::Result<Self> {
        let container = match display.windows.len() {
            1 => Container::try_from_window(&display.windows[0], display.bounds)?,
            n => {
                let widths = split_n(display.bounds.width, n);
                let xs = xs_from_widths(display.bounds.x, &widths);
                let bounds: Vec<Bounds> = widths
                    .into_iter()
                    .zip(xs.into_iter())
                    .map(|(width, x)| Bounds {
                        width,
                        x,
                        ..display.bounds
                    })
                    .collect();

                Container::try_from_windows(&display.windows, &bounds)?
            }
        };

        Ok(Self {
            // TODO: get id
            id: 0,
            bounds: display.bounds,
            root: container,
        })
    }
}

fn split_n(total: f64, n: usize) -> Vec<f64> {
    let base = total / n as f64;
    let remainder = total % n as f64;

    (0..n)
        .map(|i| {
            if (i as f64) < remainder {
                base + 1.0
            } else {
                base
            }
        })
        .collect()
}

fn xs_from_widths(start: f64, widths: &[f64]) -> Vec<f64> {
    let mut xs = Vec::with_capacity(widths.len());
    xs.push(start);
    for w in widths.iter().skip(1) {
        xs.push(xs[xs.len() - 1] + w);
    }

    xs
}
