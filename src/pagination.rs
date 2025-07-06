use serde::Serialize;

fn get_pages(
    total_pages: usize,
    current_page: usize,
    left_edge: usize,
    left_current: usize,
    right_current: usize,
    right_edge: usize,
) -> Vec<Option<usize>> {
    let last_page = total_pages;

    if last_page == 0 {
        return vec![];
    }

    let mut pages = Vec::new();

    let left_end = (1 + left_edge).min(last_page + 1);
    pages.extend((1..left_end).map(Some));

    let mid_start = left_end.max(current_page.saturating_sub(left_current));
    let mid_end = (current_page + right_current + 1).min(last_page + 1);

    if mid_start > left_end {
        pages.push(None);
    }
    pages.extend((mid_start..mid_end).map(Some));

    let right_start = mid_end.max(last_page.saturating_sub(right_edge) + 1);

    if right_start > mid_end {
        pages.push(None);
    }
    pages.extend((right_start..=last_page).map(Some));

    pages
}

#[derive(Serialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub pages: Vec<Option<usize>>,
    pub page: usize,
}

impl<T> Paginated<T> {
    pub fn new(items: Vec<T>, current_page: usize, total_pages: usize) -> Self {
        let current_page = if current_page == 0 { 1 } else { current_page };

        let pages = get_pages(total_pages, current_page, 2, 2, 4, 2);

        Self {
            items,
            pages,
            page: current_page,
        }
    }
}
