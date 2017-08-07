#[derive(Default)]
struct BlockInfo {
    toc:    bool
}

py_class!(class Block |py| {
    data toc:    BlockInfo
    def set_toc(&self, enable: bool) {
        self.toc = enable;
    }
})
