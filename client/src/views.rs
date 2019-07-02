
use cursive::view::ViewWrapper;
use cursive::views::SelectView;
use cursive::wrap_impl;
pub(crate) struct ClusterSelectView {
    view: SelectView<usize>,
}

impl ClusterSelectView {
    pub(crate) fn new() -> Self {
        ClusterSelectView {
            view: SelectView::<usize>::new(),
        }
    }
}

impl ViewWrapper for ClusterSelectView {
    wrap_impl!(self.view: SelectView<usize>);
}
