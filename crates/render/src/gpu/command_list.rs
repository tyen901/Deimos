use std::ops::Deref;

pub struct CommandList {
    command_allocator: d3d12::CommandAllocator,
    command_list: d3d12::GraphicsCommandList,
}

impl CommandList {
    pub fn new(device: &d3d12::Device) -> anyhow::Result<Self> {
        let command_allocator = device.create_command_allocator(d3d12::CommandListType::Direct)?;
        let command_list = device.create_command_list(
            0,
            d3d12::CommandListType::Direct,
            &command_allocator,
            None,
        )?;
        command_list.close()?;

        Ok(Self {
            command_allocator,
            command_list,
        })
    }

    pub fn scope<F>(&self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&d3d12::GraphicsCommandList) -> anyhow::Result<()>,
    {
        self.begin()?;
        f(&self.command_list)?;
        self.end()
    }

    pub fn begin(&self) -> anyhow::Result<()> {
        self.command_allocator.reset()?;
        self.command_list.reset(&self.command_allocator, None)?;
        Ok(())
    }

    pub fn end(&self) -> anyhow::Result<()> {
        self.command_list.close()?;
        Ok(())
    }
}

impl Deref for CommandList {
    type Target = d3d12::GraphicsCommandList;

    fn deref(&self) -> &Self::Target {
        &self.command_list
    }
}
