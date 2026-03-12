use windows::{core::Interface, Win32::Graphics::Direct3D12::*};

use crate::Device;

pub fn enable_dred() -> windows::core::Result<()> {
    unsafe {
        let mut dred_settings: Option<ID3D12DeviceRemovedExtendedDataSettings> = None;
        D3D12GetDebugInterface(&mut dred_settings)?;

        let Some(dred_settings) = dred_settings else {
            log::warn!("Auto breadcrumbs and page fault enablement not available");
            return Ok(());
        };

        // Auto-breadcrumbs track which commands were in-flight when the device was removed
        dred_settings.SetAutoBreadcrumbsEnablement(D3D12_DRED_ENABLEMENT_FORCED_ON);

        // Page fault output tells you which virtual address caused a fault
        dred_settings.SetPageFaultEnablement(D3D12_DRED_ENABLEMENT_FORCED_ON);

        log::info!("Enabled auto breadcrumbs and page fault enablement");
    }
    Ok(())
}

pub fn check_device_removed(device: &Device) {
    let removed_reason = unsafe { device.0.GetDeviceRemovedReason() };
    if removed_reason.is_ok() {
        return;
    }
    log::error!("Device removed: {:?}", removed_reason);

    let dred_data: ID3D12DeviceRemovedExtendedData1 = match device.0.cast() {
        Ok(d) => d,
        Err(e) => {
            log::error!("Failed to get DRED data: {:?}", e);
            return;
        }
    };

    unsafe {
        if let Ok(auto_breadcrumbs_output) = dred_data.GetAutoBreadcrumbsOutput1() {
            // Walk the linked list of breadcrumb nodes
            let mut node = auto_breadcrumbs_output.pHeadAutoBreadcrumbNode;
            while !node.is_null() {
                let n = &*node;
                let completed = *n.pLastBreadcrumbValue;
                let total = n.BreadcrumbCount;

                log::error!(
                    "Command queue: {} ops completed out of {}",
                    completed,
                    total
                );

                // The ops array tells you exactly which D3D12 operation was in-flight
                if !n.pCommandHistory.is_null() && total > 0 {
                    let ops = std::slice::from_raw_parts(n.pCommandHistory, total as usize);
                    // Log the last few ops around the crash point
                    let start = completed.saturating_sub(5) as usize;
                    let end = (completed as usize + 2).min(total as usize);
                    for (i, op) in ops[start..end].iter().enumerate() {
                        let idx = start + i;
                        let marker = if idx == completed as usize {
                            " <-- GPU died here"
                        } else {
                            ""
                        };
                        log::error!("  [{idx}] {op:?}{marker}");
                    }
                }

                node = n.pNext;
            }
        } else {
            log::error!("No breadcrumbs available");
        }

        if let Ok(page_fault_output) = dred_data.GetPageFaultAllocationOutput1() {
            log::error!("Page fault at GPU VA: {:#x}", page_fault_output.PageFaultVA);
            // pHeadExistingAllocationNode / pHeadRecentFreedAllocationNode
            // can tell you which resource was involved
        }
    }
}
