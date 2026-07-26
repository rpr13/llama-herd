//! Integration tests for GPU topology scanner and driver output parsers.

use llama_herd::config::is_restricted_key;
use llama_herd::discovery::{
    DriverType, GpuDevice, calculate_tensor_split, parse_nvidia_smi_output, parse_rocm_smi_output,
    parse_wddm_output, scan_gpu_topology,
};

#[test]
fn test_parse_nvidia_smi_output() {
    let sample = r"
index, name, memory.total [MiB], memory.free [MiB]
0, NVIDIA GeForce RTX 4090, 24564 MiB, 22000 MiB
1, NVIDIA RTX A6000, 49152 MiB, 45000 MiB
";

    let devices = parse_nvidia_smi_output(sample);
    assert_eq!(devices.len(), 2);

    assert_eq!(devices[0].index, 0);
    assert_eq!(devices[0].name, "NVIDIA GeForce RTX 4090");
    assert_eq!(devices[0].total_vram_mb, 24564);
    assert_eq!(devices[0].free_vram_mb, 22000);
    assert_eq!(devices[0].driver_type, DriverType::Cuda);

    assert_eq!(devices[1].index, 1);
    assert_eq!(devices[1].name, "NVIDIA RTX A6000");
    assert_eq!(devices[1].total_vram_mb, 49152);
    assert_eq!(devices[1].free_vram_mb, 45000);
    assert_eq!(devices[1].driver_type, DriverType::Cuda);
}

#[test]
fn test_parse_rocm_smi_output() {
    // Test key-value CLI format
    let kv_sample = r"
==================== ROCm System Management Interface ====================
GPU[0]		: Device Name: AMD Radeon RX 7900 XTX
GPU[0]		: VRAM Total Memory (B): 25769803776
GPU[0]		: VRAM Total Free Memory (B): 21474836480
GPU[1]		: Device Name: AMD Radeon RX 6800 XT
GPU[1]		: VRAM Total Memory (B): 17179869184
GPU[1]		: VRAM Total Used Memory (B): 4294967296
==========================================================================
";

    let devices = parse_rocm_smi_output(kv_sample);
    assert_eq!(devices.len(), 2);

    assert_eq!(devices[0].index, 0);
    assert_eq!(devices[0].name, "AMD Radeon RX 7900 XTX");
    assert_eq!(devices[0].total_vram_mb, 24576);
    assert_eq!(devices[0].free_vram_mb, 20480);
    assert_eq!(devices[0].driver_type, DriverType::Rocm);

    assert_eq!(devices[1].index, 1);
    assert_eq!(devices[1].name, "AMD Radeon RX 6800 XT");
    assert_eq!(devices[1].total_vram_mb, 16384);
    assert_eq!(devices[1].free_vram_mb, 12288);
    assert_eq!(devices[1].driver_type, DriverType::Rocm);

    // Test CSV format fallback
    let csv_sample = r"
index, name, total_vram, free_vram
0, AMD Radeon RX 7900 XTX, 24576 MiB, 20480 MiB
";
    let csv_devices = parse_rocm_smi_output(csv_sample);
    assert_eq!(csv_devices.len(), 1);
    assert_eq!(csv_devices[0].name, "AMD Radeon RX 7900 XTX");
    assert_eq!(csv_devices[0].total_vram_mb, 24576);
}

#[test]
fn test_parse_wddm_output() {
    // Test PowerShell ConvertTo-Csv output
    let csv_sample = r#"
"Name","AdapterRAM"
"NVIDIA GeForce RTX 3080","10737418240"
"Intel(R) UHD Graphics 770","1073741824"
"#;

    let devices = parse_wddm_output(csv_sample);
    assert_eq!(devices.len(), 2);

    assert_eq!(devices[0].index, 0);
    assert_eq!(devices[0].name, "NVIDIA GeForce RTX 3080");
    assert_eq!(devices[0].total_vram_mb, 10240);
    assert_eq!(devices[0].driver_type, DriverType::Wddm);

    assert_eq!(devices[1].index, 1);
    assert_eq!(devices[1].name, "Intel(R) UHD Graphics 770");
    assert_eq!(devices[1].total_vram_mb, 1024);
    assert_eq!(devices[1].driver_type, DriverType::Wddm);
}

#[test]
fn test_scan_gpu_topology_fallback() {
    let devices = scan_gpu_topology();
    // Verify scan returns a valid Vec (either empty on systems without supported GPU tools, or detected GPUs)
    for dev in &devices {
        assert!(!dev.name.is_empty());
        assert!(matches!(
            dev.driver_type,
            DriverType::Cuda | DriverType::Rocm | DriverType::Wddm | DriverType::Unknown
        ));
    }
}

#[test]
fn test_tensor_split_symmetric() {
    let gpus = vec![
        GpuDevice {
            index: 0,
            name: "GPU 0".to_owned(),
            total_vram_mb: 24576,
            free_vram_mb: 20000,
            driver_type: DriverType::Cuda,
        },
        GpuDevice {
            index: 1,
            name: "GPU 1".to_owned(),
            total_vram_mb: 24576,
            free_vram_mb: 20000,
            driver_type: DriverType::Cuda,
        },
    ];
    let split = calculate_tensor_split(&gpus, 0.12);
    assert_eq!(split, Some("1,1".to_owned()));
}

#[test]
fn test_tensor_split_asymmetric() {
    let gpus = vec![
        GpuDevice {
            index: 0,
            name: "GPU 0".to_owned(),
            total_vram_mb: 24576,
            free_vram_mb: 20000,
            driver_type: DriverType::Cuda,
        },
        GpuDevice {
            index: 1,
            name: "GPU 1".to_owned(),
            total_vram_mb: 49152,
            free_vram_mb: 40000,
            driver_type: DriverType::Cuda,
        },
    ];
    let split = calculate_tensor_split(&gpus, 0.12);
    assert_eq!(split, Some("1,2".to_owned()));
}

#[test]
fn test_tensor_split_single_gpu() {
    let single_gpu = vec![GpuDevice {
        index: 0,
        name: "GPU 0".to_owned(),
        total_vram_mb: 24576,
        free_vram_mb: 20000,
        driver_type: DriverType::Cuda,
    }];
    assert_eq!(calculate_tensor_split(&single_gpu, 0.12), None);

    let empty_gpus: Vec<GpuDevice> = vec![];
    assert_eq!(calculate_tensor_split(&empty_gpus, 0.12), None);
}

#[test]
fn test_tensor_split_three_gpus() {
    let gpus = vec![
        GpuDevice {
            index: 0,
            name: "GPU 0".to_owned(),
            total_vram_mb: 8192,
            free_vram_mb: 7000,
            driver_type: DriverType::Cuda,
        },
        GpuDevice {
            index: 1,
            name: "GPU 1".to_owned(),
            total_vram_mb: 16384,
            free_vram_mb: 14000,
            driver_type: DriverType::Cuda,
        },
        GpuDevice {
            index: 2,
            name: "GPU 2".to_owned(),
            total_vram_mb: 8192,
            free_vram_mb: 7000,
            driver_type: DriverType::Cuda,
        },
    ];
    let split = calculate_tensor_split(&gpus, 0.12);
    assert_eq!(split, Some("1,2,1".to_owned()));
}

#[test]
fn test_tensor_split_headroom_subtraction() {
    let gpus = vec![
        GpuDevice {
            index: 0,
            name: "GPU 0".to_owned(),
            total_vram_mb: 24576,
            free_vram_mb: 20000,
            driver_type: DriverType::Cuda,
        },
        GpuDevice {
            index: 1,
            name: "GPU 1".to_owned(),
            total_vram_mb: 49152,
            free_vram_mb: 40000,
            driver_type: DriverType::Cuda,
        },
    ];
    assert_eq!(calculate_tensor_split(&gpus, 0.0), Some("1,2".to_owned()));
    assert_eq!(calculate_tensor_split(&gpus, 0.50), Some("1,2".to_owned()));

    let zero_vram = vec![
        GpuDevice {
            index: 0,
            name: "GPU 0".to_owned(),
            total_vram_mb: 0,
            free_vram_mb: 0,
            driver_type: DriverType::Cuda,
        },
        GpuDevice {
            index: 1,
            name: "GPU 1".to_owned(),
            total_vram_mb: 0,
            free_vram_mb: 0,
            driver_type: DriverType::Cuda,
        },
    ];
    assert_eq!(calculate_tensor_split(&zero_vram, 0.12), None);
}

#[test]
fn test_restricted_keys_tensor_split() {
    assert!(is_restricted_key("tensor-split"));
    assert!(is_restricted_key("fit"));
    assert!(is_restricted_key("fitt"));
}
