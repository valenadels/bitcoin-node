use gtk::{
    prelude::*, Box, Builder, Button, Fixed as GtkFixed, Label, ListBox, Popover, ProgressBar,
    ScrolledWindow, Widget,
};

use crate::{
    block_header::BlockHeader,
    constants::COMPLETE_DOWNLOAD_FRACTION,
    node_error::NodeError,
    ui::utils::{build_block_info, get_object_by_name, timestamp_to_date, u8_to_hex_string},
};

/// BlockExplorerPage shows all the block hashes from the timestamp defined in config
pub struct BlockExplorerPage {
    /// The page that holds all the elements
    pub page: GtkFixed,
    /// The builder to build the page
    builder: Builder,
    // The list box that holds all the block hashes
    list_box: ListBox,
    /// The progress bar that shows the block download progress
    progress_bar: ProgressBar,
    /// The total number of blocks downloaded
    blocks_count: i64,
    /// The total number of batch headers
    headers_count: i64,
    /// The total number of blocks to download
    total_blocks: i64,
    /// The label that shows the title Blocks Download Status
    label_blocks: Label,
    /// The starting date timestamp
    timestamp: Label,
    /// The label that shows the progress of the headers download
    label_headers: Label,
    /// The icon that shows the loading of the headers
    icon_loading: gtk::Spinner,
}

impl BlockExplorerPage {
    /// Creates a new BlockExplorerPage
    /// # Arguments
    /// * `child` - The child widget that holds the page
    /// * `builder` - The builder to build the page
    /// # Returns
    /// * `Result<BlockExplorerPage, NodeError>` - The result of the creation
    pub fn new(child: Widget, builder: Builder) -> Result<Self, NodeError> {
        let page = child
            .downcast::<GtkFixed>()
            .map_err(|_| NodeError::UIError("Failed to downcast to GtkFixed".to_string()))?;
        let list_box = gtk::ListBox::new();
        let progress_bar: ProgressBar = get_object_by_name(&builder, "download_status_bar")?;
        progress_bar.hide();
        let timestamp = get_object_by_name(&builder, "timestamp")?;
        let label_headers = get_object_by_name(&builder, "headers_progress")?;
        let icon_loading = get_object_by_name(&builder, "headers_loading")?;
        let label_blocks: Label = get_object_by_name(&builder, "status_label")?;
        label_blocks.hide();

        Ok(BlockExplorerPage {
            page,
            builder,
            list_box,
            progress_bar,
            blocks_count: 0,
            total_blocks: 0,
            label_blocks,
            headers_count: 0,
            timestamp,
            label_headers,
            icon_loading,
        })
    }

    /// Shows how many headers are being downloaded (in chunks of 2000)
    pub fn show_loading_headers(&mut self) {
        self.headers_count += 1;
        self.label_headers
            .set_text(&format!("Chunk of headers count: {}", self.headers_count));
    }

    /// Hides the loading headers and shows the progress bar
    pub fn hide_loading_headers(&mut self) {
        self.icon_loading.hide();
        self.label_headers.hide();
        self.progress_bar.show();
        self.label_blocks.show();
    }

    /// Sets the starting date timestamp label
    pub fn set_starting_date(&mut self, starting_date: String) {
        let timestamp_date =
            timestamp_to_date(starting_date.parse::<u32>().unwrap_or(0)).unwrap_or("".to_string());
        self.timestamp
            .set_text(&format!("Blocks since {}", &timestamp_date));
    }

    /// Sets the total number of blocks to download
    pub fn set_total_blocks(&mut self, total_blocks: i64) {
        self.total_blocks = total_blocks;
    }

    /// Adds a widget to the page
    /// # Arguments
    /// * `element` - The element to add which implements IsA<Widget>
    pub fn add(&self, element: &impl IsA<Widget>) {
        self.page.add(element);
    }

    /// Builds the list box with all the block hashes
    /// # Arguments
    /// * `block_headers` - The block headers to build the list box with
    /// # Returns
    /// * `Result<(), NodeError>` - The result of the building
    pub fn build_list_box(&self, block_headers: Vec<BlockHeader>) -> Result<(), NodeError> {
        let scrolled_window: ScrolledWindow =
            get_object_by_name(&self.builder, "scrolled_view_block")?;

        self.build_list_block_from_headers(block_headers);
        self.list_box.show_all();
        let box_layout = Box::new(gtk::Orientation::Vertical, 0);
        scrolled_window.add(&self.list_box);
        scrolled_window.queue_resize();
        box_layout.pack_start(&scrolled_window, true, true, 0);

        self.add(&box_layout);
        Ok(())
    }

    /// Updates the progress bar by adding a new block to the count
    pub fn increment_progress_bar(&mut self) {
        self.blocks_count += 1;
        if self.total_blocks > 0 {
            let fraction = self.blocks_count as f64 / self.total_blocks as f64;
            self.progress_bar.set_fraction(fraction);
            self.progress_bar.set_text(Some(&format!(
                "{}/{}",
                self.blocks_count, self.total_blocks
            )));
            if fraction == COMPLETE_DOWNLOAD_FRACTION {
                self.progress_bar.set_text(Some(&format!(
                    "Finished download. Total: {}",
                    self.total_blocks
                )))
            }
        }
    }

    /// Adds a block header to the list box
    /// # Arguments
    /// * `block_header` - The block header to add
    pub fn add_new_block_received(&self, block_header: BlockHeader) {
        self.add_block(block_header);
        self.list_box.show_all();
    }

    /// Adds a row to the list box containing the block header
    /// # Arguments
    /// * `block_header` - The block header to add
    fn add_block(&self, block_header: BlockHeader) {
        let row = gtk::ListBoxRow::new();
        let mut cloned_hash = block_header.hash.clone();
        cloned_hash.reverse();
        let hash = u8_to_hex_string(&cloned_hash);
        let button_label = Button::new();
        button_label.set_label(&hash);
        let cloned_row = row.clone();
        button_label.connect_clicked(move |_| {
            let popover = Popover::new(Some(&cloned_row));
            let popover_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
            let block_info = build_block_info(&block_header);
            popover_box.add(&block_info);
            popover.add(&popover_box);
            popover.show_all();
            popover.set_position(gtk::PositionType::Bottom);
            popover.set_modal(true);
            popover.set_relative_to(Some(&cloned_row));
        });

        row.set_margin_bottom(10);
        row.add(&button_label);
        self.list_box.add(&row);
    }

    /// Builds a list of blocks from block headers in a GTK list box.
    ///
    /// # Arguments
    ///
    /// * `block_headers` - The block headers to build the list from.
    /// * `list_box` - A reference to the `gtk::ListBox` where the list will be built.
    ///
    /// # Remarks
    ///
    /// This function iterates over the `block_headers` and creates a `gtk::ListBoxRow` for each block header.
    /// It constructs a button with the block's hash as the label, and attaches a popover to show block information on button click.
    /// The constructed rows are added to the `list_box`.
    fn build_list_block_from_headers(&self, block_headers: Vec<BlockHeader>) {
        for item in block_headers {
            self.add_block(item);
        }
        self.list_box.show_all();
    }
}
