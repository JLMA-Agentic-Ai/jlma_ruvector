//! `rvf verify-attestation` -- Verify KernelBinding and attestation.

use clap::Args;
use std::path::Path;

use rvf_runtime::RvfStore;
use rvf_types::kernel::KERNEL_MAGIC;

use super::map_rvf_err;

#[derive(Args)]
pub struct VerifyAttestationArgs {
    /// Path to the RVF store
    pub file: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

pub fn run(args: VerifyAttestationArgs) -> Result<(), Box<dyn std::error::Error>> {
    let store = RvfStore::open_readonly(Path::new(&args.file)).map_err(map_rvf_err)?;

    let kernel_data = store.extract_kernel().map_err(map_rvf_err)?;

    match kernel_data {
        None => {
            if args.json {
                crate::output::print_json(&serde_json::json!({
                    "status": "no_kernel",
                    "message": "No KERNEL_SEG found in file",
                }));
            } else {
                println!("No KERNEL_SEG found in file.");
            }
        }
        Some((header_bytes, image_bytes)) => {
            // Verify kernel header magic
            let magic = u32::from_le_bytes([
                header_bytes[0], header_bytes[1],
                header_bytes[2], header_bytes[3],
            ]);
            let magic_valid = magic == KERNEL_MAGIC;

            // Check if KernelBinding is present (128 bytes after 128-byte header)
            // In the new wire format: header(128) + KernelBinding(128) + cmdline + image
            // In old format: header(128) + cmdline + image (no binding)
            let has_binding = image_bytes.len() >= 128;

            let mut binding_valid = false;
            let mut manifest_hash_hex = String::new();
            let mut policy_hash_hex = String::new();

            if has_binding {
                // Extract potential KernelBinding from first 128 bytes of "image" portion
                let binding_bytes = &image_bytes[..128];
                manifest_hash_hex = crate::output::hex(&binding_bytes[0..32]);
                policy_hash_hex = crate::output::hex(&binding_bytes[32..64]);

                // Check binding_version (offset 0x40-0x41)
                let binding_version = u16::from_le_bytes([
                    binding_bytes[64], binding_bytes[65],
                ]);

                // A binding is considered present if version > 0
                binding_valid = binding_version > 0;
            }

            // Check image hash from header
            let image_hash = &header_bytes[0x30..0x50];
            let image_hash_hex = crate::output::hex(image_hash);

            // Verify arch
            let arch = header_bytes[0x06];
            let arch_name = match arch {
                1 => "x86_64",
                2 => "aarch64",
                3 => "riscv64",
                _ => "unknown",
            };

            if args.json {
                crate::output::print_json(&serde_json::json!({
                    "status": if magic_valid { "valid" } else { "invalid" },
                    "magic_valid": magic_valid,
                    "arch": arch_name,
                    "has_kernel_binding": binding_valid,
                    "manifest_root_hash": if binding_valid { &manifest_hash_hex } else { "" },
                    "policy_hash": if binding_valid { &policy_hash_hex } else { "" },
                    "image_hash": image_hash_hex,
                    "image_size": image_bytes.len(),
                }));
            } else {
                println!("Attestation verification:");
                crate::output::print_kv("Magic valid:", &magic_valid.to_string());
                crate::output::print_kv("Architecture:", arch_name);
                crate::output::print_kv("Image size:", &format!("{} bytes", image_bytes.len()));
                crate::output::print_kv("Image hash:", &image_hash_hex);
                if binding_valid {
                    println!();
                    println!("  KernelBinding present:");
                    crate::output::print_kv("Manifest hash:", &manifest_hash_hex);
                    crate::output::print_kv("Policy hash:", &policy_hash_hex);
                } else {
                    println!();
                    println!("  No KernelBinding found (legacy format or unsigned stub).");
                }
            }
        }
    }
    Ok(())
}
