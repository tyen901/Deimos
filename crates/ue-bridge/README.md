# Unreal source data bridge

This crate consumes installed Marathon packages through pinned tiger-pkg. It does not create a window, a renderer, D3D12 resources, or Unreal objects. Unreal owns task scheduling and all UObject/RHI lifetimes.

The bridge was developed for Mara3, using Deimos package/buffer/technique behavior as format evidence. Its decoder and boundary originate in Mara3's direct-loading implementation (commit 7e4d0d9). No game payload belongs in this repository.

The C-callable functions return owned memory plus explicit status; the host exports these functions from its own DLL and frees returned buffers through mara_free. Open returns an opaque PackageManager pointer. Close requires all readers to have completed. Opening indexes packages with two bounded workers; later reads run on the host's bounded worker queue. tag32 and tag64 identities retain their source values.

mara_character resolves one entity's render component, skeleton, attachments, model parts, stage ranges, category masks, buffer identities, and skin influences directly. It returns a memory-only graph; no intermediate file or imported asset is required. mara_tag and mara_entry support exact source reads and inspection; mara_resolve64 consults the actual source lookup table.

Native animation codecs 1/2/3 and source rig records are decoded independently of the renderer. Complete rig execution, material/shader translation and global dependency/residency control remain unfinished. The existing Deimos renderer is not a source animation solution and is not linked by this crate.

`mara_equipment` reads an exact node-table entry and its source configuration, evaluates BACC/BAD0 switch conditions, and resolves the selected equipment assembly. Skeleton-owning equipment and its 032B3 render children remain separate nodes with explicit skeleton ownership. The installed 032B3 child table is at +118; older schema offsets do not apply. Missing primary motion tables are represented as absent source relationships, independently of the clip bank. Unsupported selection or hierarchy fails explicitly.
