use yew::{Html, html};

use crate::net::UseHttpHandleOptionRender;

pub(super) fn render(ctx: &super::Context) -> Html {
    // properties
    let super::RouteProps {
        conf: _,
        drawer_id: _,
        i18n,
    } = ctx.props.route.clone();

    let dir_entry = ctx.file_entry.ok();
    let dir_files = dir_entry.map(|dir| &dir.files);
    let file_entry = ctx
        .selected_entry
        .and_then(|index| dir_files.and_then(|files| files.get(index)));

    let is_dir = file_entry.is_none_or(|file| file.is_dir());
    let ty = file_entry.and_then(|file| file.ty());

    html! {
        <aside
            class={ format!(
                "w-screen lg:w-96 h-full transition {}",
                if file_entry.is_some() { "" } else { "hidden" }
            ) }
        >
            <div class="flex flex-col w-screen lg:w-96 h-full overflow-y-none bg-slate-50 border-r border-gray-200 border-l-2 border-l-gray-200">
                // File brief
                <div class="flex items-start justify-between px-4 pt-5 pb-3">
                    // File Icon
                    <div class="mr-4">{{
                        let color = None;
                        let fill = true;
                        let size = 6;
                        super::mime::render_file_entry(ty, is_dir, color, fill, size)
                    }}</div>

                    // File name
                    <h2 class="flex-1 line-clamp-6 mr-4 select-none text-wrap wrap-anywhere">{
                        for file_entry.map(|file| file.name.clone())
                    }</h2>

                    // Close button
                    <button
                        class="btn btn-sm btn-circle"
                        onclick={{
                            let selected = ctx.selected_entry.clone();
                            move |_| selected.set(None)
                        }}
                    >{ "✕" }</button>
                </div>

                <div class="divider my-0" />

                // File details
                <div class="flex justify-between items-center p-4">
                    <h3 class="font-bold select-none">{ "파일 세부정보" }</h3>
                </div>
                <div class="px-4 overflow-y-auto">
                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Size" }</span>
                        </label>
                        <p class="line-clamp-2 text-wrap wrap-anywhere">{{
                            let size = file_entry.and_then(|file| file.metadata.size);
                            i18n.format_size(is_dir, size)
                        }}</p>
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Owner" }</span>
                        </label>
                        <p class="line-clamp-2 text-wrap wrap-anywhere">{
                            for file_entry
                                .and_then(|file| file.metadata.owner.as_ref())
                                .map(|owner| owner.name.clone())
                        }</p>
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Modified" }</span>
                        </label>
                        <p
                            class="line-clamp-2 tooltip text-wrap wrap-anywhere"
                            data-tip={ "22 minutes ago by me" }
                        >{ "22 minutes ago by me" }</p>
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Accessed" }</span>
                        </label>
                        <p
                            class="line-clamp-2 tooltip text-wrap wrap-anywhere"
                            data-tip={ "22 minutes ago by me" }
                        >{ "22 minutes ago by me" }</p>
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Created" }</span>
                        </label>
                        <p
                            class="line-clamp-2 tooltip text-wrap wrap-anywhere"
                            data-tip={ "22 minutes ago by me" }
                        >{ "22 minutes ago by me" }</p>
                    </div>

                    <div class="divider mt-0 mb-1" />
                    <h3 class="font-bold select-none text-sm mb-2">{ "사용자가 추가함" }</h3>

                    <div class="form-control pb-4">
                        <label class="flex items-start label p-0 mb-1">
                            <span class="flex-1 label-text-alt text-sm">{ "Quality" }</span>
                            <button class="btn btn-sm btn-circle w-6 h-6 mr-2">{ "✕" }</button>
                        </label>
                        <input
                            type="text"
                            value="Silver"
                            class="input input-bordered input-sm w-full"
                            disabled=false
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="flex items-start label p-0 mb-1">
                            <span class="flex-1 label-text-alt text-sm">{ "Catalog Name" }</span>
                            <button class="btn btn-sm btn-circle w-6 h-6 mr-2">{ "✕" }</button>
                        </label>
                        <input
                            type="text"
                            value="pdfs"
                            class="input input-bordered input-sm w-full"
                            disabled=false
                        />
                    </div>

                    <div class="divider mt-0 mb-1" />
                    <h3 class="font-bold select-none text-sm mb-2">{ "시스템에 설정함" }</h3>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Minimal Storage Bandwidth" }</span>
                        </label>
                        <input
                            type="text"
                            value="10 Gbps"
                            class="input input-bordered input-sm w-full"
                            disabled=false
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Maximal Storage Latency" }</span>
                        </label>
                        <input
                            type="text"
                            value="10 ms"
                            class="input input-bordered input-sm w-full"
                            disabled=false
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Preferred Computing" }</span>
                        </label>
                        <select class="select select-bordered select-sm w-full">
                            <option selected=true>{ "Balanced" }</option>
                            <option>{ "Max Bandwidth" }</option>
                            <option>{ "Min Latency" }</option>
                        </select>
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Preferred Storage" }</span>
                        </label>
                        <select class="select select-bordered select-sm w-full">
                            <option selected=true>{ "Automatic" }</option>
                            <option>{ "Hot Storage (NVMe)" }</option>
                            <option>{ "Warm Storage (HDD)" }</option>
                        </select>
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Backup Policy" }</span>
                        </label>
                        <select class="select select-bordered select-sm w-full">
                            <option selected=true>{ "None" }</option>
                            <option>{ "Daily" }</option>
                            <option>{ "Weekly" }</option>
                            <option>{ "Monthly" }</option>
                            <option>{ "Annually" }</option>
                        </select>
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Computing Plane" }</span>
                        </label>
                        <select class="select select-bordered select-sm w-full">
                            <option selected=true>{ "Automatic" }</option>
                            <option>{ "Frontend" }</option>
                            <option>{ "Backend" }</option>
                            <option>{ "Storage" }</option>
                            <option>{ "Device" }</option>
                        </select>
                    </div>

                    <div class="divider mt-0 mb-1" />
                    <h3 class="font-bold select-none text-sm mb-2">{ "시스템에 설정됨" }</h3>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Computing Engine" }</span>
                        </label>
                        <input
                            type="text"
                            value="Connected Data Lake (WASM Direct)"
                            class="input input-bordered input-sm w-full tooltip"
                            data-tip="Connected Data Lake (WASM Direct)"
                            disabled=true
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Asynchronous Runtime" }</span>
                        </label>
                        <input
                            type="text"
                            value="Linux (io_uring)"
                            class="input input-bordered input-sm w-full tooltip"
                            data-tip="Linux (io_uring)"
                            disabled=true
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Data Transfer Protocol" }</span>
                        </label>
                        <input
                            type="text"
                            value="Ethernet (100Gbps)"
                            class="input input-bordered input-sm w-full tooltip"
                            data-tip="Ethernet (100Gbps)"
                            disabled=true
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Filesystem" }</span>
                        </label>
                        <input
                            type="text"
                            value="EXT4"
                            class="input input-bordered input-sm w-full tooltip"
                            data-tip="EXT4"
                            disabled=true
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Storage Protocol" }</span>
                        </label>
                        <input
                            type="text"
                            value="PCIe (4.0)"
                            class="input input-bordered input-sm w-full tooltip"
                            data-tip="PCIe (4.0)"
                            disabled=true
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Storage Protocol" }</span>
                        </label>
                        <input
                            type="text"
                            value="NVMe (Core)"
                            class="input input-bordered input-sm w-full tooltip"
                            data-tip="NVMe (Core)"
                            disabled=true
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "Storage Driver" }</span>
                        </label>
                        <input
                            type="text"
                            value="Linux (nvme)"
                            class="input input-bordered input-sm w-full tooltip"
                            data-tip="Linux (nvme)"
                            disabled=true
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "NVMe Device SN" }</span>
                        </label>
                        <input
                            type="text"
                            value="HIDDEN"
                            class="input input-bordered input-sm w-full tooltip"
                            data-tip="HIDDEN"
                            disabled=true
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "NVMe Device Model" }</span>
                        </label>
                        <input
                            type="text"
                            value="HIDDEN"
                            class="input input-bordered input-sm w-full tooltip"
                            data-tip="HIDDEN"
                            disabled=true
                        />
                    </div>

                    <div class="form-control pb-4">
                        <label class="label p-0 mb-1">
                            <span class="label-text-alt text-sm">{ "NVMe Device Namespace" }</span>
                        </label>
                        <input
                            type="text"
                            value="0x1"
                            class="input input-bordered input-sm w-full tooltip"
                            data-tip="0x1"
                            disabled=true
                        />
                    </div>
                </div>

                <div class="divider my-0 bg-gray-100" />

                // Add a tag
                <div class="join m-0 px-1 pb-1 bg-gray-100 w-full">
                    <input
                        class="input input-bordered input-sm w-full join-item"
                        placeholder="새 태그 입력..."
                    />
                    <button class="btn btn-sm join-item">{ "추가" }</button>
                </div>
            </div>
        </aside>
    }
}
