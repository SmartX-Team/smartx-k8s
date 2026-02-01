# OpenARK VINE Browser Backend

A reference implementation of `OpenARK VINE Browser` backend.
I've modularized this backend to a minimal set of features so that it can be accelerated across multiple layers, as shown below.

1. `[Frontend Layer]`: A user-friendly GUI environment. [A minimal implementation is here.](../openark-vine-browser-frontend/README.md)

2. `[K8S Layer]` **"Connectable Data Lake"**: L7 middleware leveraging the `K8S`. Security such as TLS can be implemented without separate implementation. Furthermore, by exchanging the physical and logical addresses of datasets, the location of the dataset can be virtualized and dataset portability can be secured.

   - **Purpose: A Control Plane for Data and Backends** Leveraging `K8S`
   - _Upper Layer_: `Frontend` Layer
   - _Lower Layer_: `Load-Balancing` Layer
   - Features
     - TLS Layer (Interconnection Security)
     - Data Virtualization
     - Scale-Out

3. `[Load-Balancing Layer]` **"Accelerated Data Lake"**: Selects the optimal backend target in real-time to fit the user's budget. It optimizes the overall `TCO` of the system by adjusting the asynchronous runtime, storage abstraction level, the presence and quantity of dedicated hardware, and internal protocols (Ethernet vs. RDMA).

   - **Purpose: Cost-Efficient Data Processing** Leveraging `WASM` and `io_uring`
   - _Upper Layer_: `K8S` Layer
   - _Lower Layer_: `WASM` Layer
   - Features
     - Adaptive Load-Balancing
     - Total Cost Optimization
     - Hybrid Storage Backends

4. `[WASM Layer]` **"Connected Data Lake"**: User-defined functions can be implemented using WebAssembly. Server-side computations can be performed based on user-required data and dataset formats, such as S3 and NFS protocols. Lightweight, high-performance I/O is possible by utilizing the latest asynchronous runtimes such as `io_uring`.

   - **Purpose: User-Defined Data Processing** Leveraging `WASM` and `io_uring`
   - _Upper Layer_: `Load-Banacing` Layer
   - _Lower Layer_: `Storage` Layer
   - Features
     - _Compute-over-Storage_ with WASM Functions
     - Computational Object Storage
     - Shared asynchronous runtimes (i.e. `io_uring`)

5. `[Storage Layer]`: Exposes storage layers such as NVMe to the `WASM layer`. Users can utilize not only the existing abstracted POSIX, but also dedicated hardware such as NVMe and the latest command sets to implement processing methods appropriate to the purpose and format of the data. If the user has permission, they can opt for a dedicated, polling-based runtime instead of an `io_uring`-based shared asynchronous runtime.

   - **Purpose: High-Performance Data Processing** Leveraging `NVMe` and polling
   - _Upper Layer_: `WASM` Layer
   - _Lower Layer_: `H/W` Layer (HAL)
   - Features
     - NVMe Storage
     - `#![no_std]` Support: All layers can be utilized to develop new kernels or drivers for existing kernels, or as a framework for dedicated semiconductors, such as `DPU`s, for hardware acceleration of specific layers.
     - Dedicated asynchronous runtimes (i.e. CPU pinning & Polling)

[For more information, please visit our latest storage acceleration framework.](https://github.com/SmartX-Team/Connected-Data-Lake)

## Getting Started

In your development machine, please type this command below:

```bash
just run-openark-vine-browser-backend
```

## Research Steps

The following layers are being studied under the following names:

1. Connectable Data Lake -> `K8S Layer` (KCI-under-review)
2. Connected Data Lake -> `WASM Layer` (SCI-under-review)
3. Accelerated Data Lake -> `Load-Balancing Layer`

## License

Please refer the [LICENSE](LICENSE) file.
