use std::{process::Stdio, sync::Arc};

use clap::Parser;
use filetime::FileTime;
use human_bytes::human_bytes;
use indicatif::{ProgressBar, ProgressStyle};
use tokio::{sync::Semaphore, task::JoinSet};

#[derive(Parser, Clone)]
struct Args {
    #[clap(short, long)]
    input: String,

    #[clap(short, long)]
    output: String,

    #[clap(short, long)]
    recursive: bool,

    #[clap(short, long, default_value_t = 2)]
    jobs: usize,

    #[clap(short, long)]
    copy_all: bool,

    #[clap(short, long, default_value_t = 7)]
    effort: u32,

    #[clap(short, long)]
    yes: bool,
}

const ACCEPTED_EXTENSIONS: &[&str] = &[
    // Joint Photographic Experts Group
    "jpg",
    "jpeg",
    "jpe",
    "jif",
    "jfif",
    "jfi",
    // Portable Network Graphics
    "png",
    // WebP
    "webp",
    // Graphics Interchange Format
    "gif",
    // Bitmap
    "bmp",
    "dib",
    // Portable Pixmap
    "ppm",
    // Portable Graymap
    "pgm",
    // Portable Anymap
    "pam",
    // Tagged Image File Format
    "tif",
    "tiff",
    // Targa Graphics Format
    "tga",
    "icb",
    "vda",
    "vst",
    // DirectDraw Surface
    "dds",
    // OpenEXR
    "exr",
    // High Dynamic Range
    "hdr",
    "pic",
    // Microsoft Icon
    "ico",
    // Flexible Image Transport System
    "fits",
    // Alias/Wavefront PIX image
    "pix",
    // BRender PIX image
    "brender_pix",
    // CompuServe Graphics Interchange Format (GIF) - already included
    // piped gif sequence - already included
    // piped hdr sequence - already included
    // Microsoft Windows ICO - already included
    // piped j2k sequence (JPEG 2000)
    "j2k",
    "jp2",
    "jpt",
    // piped jpeg sequence - already included
    // piped jpegls sequence (JPEG-LS)
    "jls",
    // Animated JPEG XL - already included
    // piped jpegxl sequence - already included
    // piped pam sequence - already included
    // piped pbm sequence (Portable Bitmap)
    "pbm",
    // piped pcx sequence (PC Paintbrush)
    "pcx",
    // piped pfm sequence (Portable FloatMap)
    "pfm",
    // piped pgm sequence - already included
    // piped pgmyuv sequence (Portable Graymap with YUV)
    "pgmyuv",
    // piped pgx sequence (JPEG 2000 part 1)
    "pgx",
    // piped phm sequence (Portable Half FloatMap)
    "phm",
    // piped photocd sequence (Photo CD)
    "pcd",
    // piped pictor sequence (Pictor)
    "pct",
    "pict",
    // piped png sequence - already included
    // piped ppm sequence - already included
    // piped psd sequence (Adobe Photoshop)
    "psd",
    // piped qdraw sequence (QuickDraw)
    "qdraw",
    // piped qoi sequence (Quite OK Image)
    "qoi",
    // piped sgi sequence (Silicon Graphics Image)
    "sgi",
    // piped sunrast sequence (Sun Raster)
    "ras",
    // piped tiff sequence - already included
    // piped vbn sequence (VBN)
    "vbn",
    // piped webp sequence - already included
    // piped xbm sequence (XBM)
    "xbm",
    // piped xpm sequence (XPM)
    "xpm",
    // piped xwd sequence (XWD)
    "xwd",
];

enum ProcessResult {
    Converted {
        original_size: u64,
        converted_size: u64,
    },
    Copied,
    Skipped,
    Error(anyhow::Error),
}

async fn convert_image(
    input_path: &std::path::Path,
    output_file_path: &std::path::Path,
    effort: u32,
) -> anyhow::Result<(u64, u64)> {
    // Changed return type
    println!(
        "   Converting {} -> {}",
        input_path.display(),
        output_file_path.display()
    );

    // Convert the image to JXL format using ffmpeg.
    let mut process = tokio::process::Command::new("ffmpeg")
        .arg("-i")
        .arg(input_path)
        .arg("-map")
        .arg("0")
        .arg("-c:v")
        .arg("libjxl")
        // .arg("-lossless") // Lossless compression
        .arg("-effort")
        .arg(effort.to_string()) // Compression effort (1-9)
        .arg("-map_metadata")
        .arg("0") // Copy metadata from input to output
        .arg(&output_file_path)
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let status = process.wait().await?;
    if !status.success() {
        return Err(anyhow::anyhow!("Failed to convert image"));
    }

    let src_fs_metadata = std::fs::metadata(input_path)?;
    let modified_timestamp = src_fs_metadata.modified()?;

    println!(
        "      Setting modified timestamp to {:?}",
        modified_timestamp
    );
    filetime::set_file_mtime(
        &output_file_path,
        FileTime::from_last_modification_time(&src_fs_metadata),
    )?;

    let src_size = src_fs_metadata.len();
    let output_file_path = std::path::PathBuf::from(&output_file_path);
    let dst_size = std::fs::metadata(output_file_path)?.len();

    println!(
        "      Compressed from {} -> {}",
        human_bytes(src_size as f64),
        human_bytes(dst_size as f64)
    );

    Ok((src_size, dst_size)) // Return sizes
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let input_path = std::path::PathBuf::from(&args.input);
    if !input_path.exists() {
        return Err(anyhow::anyhow!("Input path does not exist"));
    }
    if !input_path.is_dir() {
        return Err(anyhow::anyhow!("Input path is not a directory"));
    }

    let output_path = std::path::PathBuf::from(&args.output);
    if !output_path.exists() {
        std::fs::create_dir_all(&output_path)?;
    }
    if !output_path.is_dir() {
        return Err(anyhow::anyhow!("Output path is not a directory"));
    }

    let mut walkdir = walkdir::WalkDir::new(&input_path);
    if !args.recursive {
        walkdir = walkdir.max_depth(1);
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} [{elapsed_precise}] {msg}")
            .unwrap(),
    );
    pb.set_message("Collecting files...");

    let files_to_process = walkdir
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            if args.copy_all {
                // If copy_all is true, include all files
                true
            } else {
                // Otherwise, only include accepted image extensions
                let extension = e
                    .path()
                    .extension()
                    .and_then(std::ffi::OsStr::to_str)
                    .unwrap_or("")
                    .to_lowercase();
                ACCEPTED_EXTENSIONS.contains(&extension.as_str())
            }
        })
        .map(|e| {
            pb.inc(1);
            e.path().to_owned()
        })
        .collect::<Vec<_>>();

    pb.finish_with_message(format!("Collected {} files.", files_to_process.len()));

    // Initial calculation of total source size for files that will be processed
    // This is used for the initial overview printout.
    let initial_processed_files_size = files_to_process
        .iter()
        .fold(0, |acc, f| acc + std::fs::metadata(f).unwrap().len());

    // Print a nice overview of what is going to happen
    println!("{}", "-".repeat(60)); // Simple separator

    // Calculate padding for alignment
    let labels = [
        "Input",
        "Output",
        "Recursive",
        "Jobs",
        "Copy All",
        "Files to process",
    ];
    let max_label_width = labels.iter().map(|s| s.len()).max().unwrap_or(0);

    // Print aligned key-value pairs
    println!(
        "{:<width$} : {}",
        "Input",
        args.input,
        width = max_label_width
    );
    println!(
        "{:<width$} : {}",
        "Output",
        args.output,
        width = max_label_width
    );
    println!(
        "{:<width$} : {}",
        "Recursive",
        if args.recursive { "Yes" } else { "No" },
        width = max_label_width
    );
    println!(
        "{:<width$} : {}",
        "Jobs",
        args.jobs,
        width = max_label_width
    );
    println!(
        "{:<width$} : {}",
        "Copy All",
        if args.copy_all { "Yes" } else { "No" },
        width = max_label_width
    );
    println!(
        "{:<width$} : {} (Total size: {})",
        "Files to process",
        files_to_process.len(),
        human_bytes::human_bytes(initial_processed_files_size as f64),
        width = max_label_width
    );

    println!("{}", "-".repeat(60)); // Simple separator
    println!(); // Add a blank line for spacing

    // Ask the user wether they are sure to proceed
    if args.yes {
        let confirmation = inquire::Confirm::new("Are you sure to proceed?")
            .with_default(false)
            .prompt()?;

        if !confirmation {
            println!("Aborting...");
            return Ok(());
        }
    }

    let mut set: JoinSet<anyhow::Result<ProcessResult>> = JoinSet::new(); // Updated JoinSet return type
    let semaphore = Arc::new(Semaphore::new(args.jobs));

    let total_files_to_process = files_to_process.len(); // Use the new variable
    let mut completed_count = 0;
    let mut converted_count = 0; // Track converted files
    let mut copied_count = 0; // Track copied files
    let mut skipped_count = 0; // Track skipped files
    let mut error_count = 0; // Track errors

    // Initialize total size counters for converted files
    let mut total_original_size: u64 = 0;
    let mut total_converted_size: u64 = 0;

    for file in files_to_process {
        let semaphore = semaphore.clone();
        let output_base_path = output_path.clone();
        let input_base_path = input_path.clone();
        let args = args.clone(); // Clone args for use in the async block

        set.spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();

            let relative_path = file.strip_prefix(&input_base_path)?;
            let file_extension = file
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or("")
                .to_lowercase();

            let effort = args.effort;

            if ACCEPTED_EXTENSIONS.contains(&file_extension.as_str()) {
                // This is an image file, attempt conversion
                let output_file_path = output_base_path.join(relative_path).with_extension("jxl");

                if output_file_path.exists() {
                    println!("   Skipping existing JXL: {}", output_file_path.display());
                    return Ok(ProcessResult::Skipped);
                }

                if let Some(parent) = output_file_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                // Call convert_image and get the sizes
                match convert_image(&file, &output_file_path, effort).await {
                    Ok((original_size, converted_size)) => Ok(ProcessResult::Converted {
                        original_size,
                        converted_size,
                    }),
                    Err(e) => Ok(ProcessResult::Error(e)), // Wrap error in ProcessResult
                }
            } else if args.copy_all {
                // This is a non-image file and copy_all is true, attempt copy
                let output_file_path = output_base_path.join(relative_path);

                if output_file_path.exists() {
                    println!("   Skipping existing file: {}", output_file_path.display());
                    return Ok(ProcessResult::Skipped);
                }

                if let Some(parent) = output_file_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                println!(
                    "   Copying {} -> {}",
                    file.display(),
                    output_file_path.display()
                );
                match tokio::fs::copy(&file, &output_file_path).await {
                    Ok(_) => Ok(ProcessResult::Copied),
                    Err(e) => Ok(ProcessResult::Error(anyhow::anyhow!("Copy failed: {}", e))), // Wrap copy error
                }
            } else {
                // This is a non-image file and copy_all is false, skip
                println!("   Skipping non-image file: {}", file.display());
                return Ok(ProcessResult::Skipped);
            }
        });
    }

    while let Some(task_result) = set.join_next().await {
        completed_count += 1; // Increment completed count regardless of task outcome

        match task_result {
            // Handle the Result from the spawned task (Result<anyhow::Result<ProcessResult>, tokio::task::JoinError>)
            Ok(process_result_wrapped) => {
                // Task completed successfully, result is anyhow::Result<ProcessResult>
                match process_result_wrapped {
                    // Now match on the anyhow::Result<ProcessResult>
                    Ok(process_result) => {
                        // Task returned Ok(ProcessResult)
                        match process_result {
                            // Now match on the inner ProcessResult enum
                            ProcessResult::Converted {
                                original_size,
                                converted_size,
                            } => {
                                converted_count += 1;
                                total_original_size += original_size;
                                total_converted_size += converted_size;
                            }
                            ProcessResult::Copied => {
                                copied_count += 1;
                            }
                            ProcessResult::Skipped => {
                                skipped_count += 1;
                            }
                            ProcessResult::Error(e) => {
                                eprintln!("Error processing file: {}", e);
                                error_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        // Task returned Err(anyhow::Error)
                        eprintln!("Error processing file: {}", e);
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                // This branch handles errors from join_next() (e.g., task panic)
                eprintln!("Task join error: {}", e);
                error_count += 1; // Count join errors as well
            }
        }

        println!(
            "Progress: {}/{} files processed",
            completed_count, total_files_to_process
        );
    }

    // Calculate and print the final summary
    println!("{}", "-".repeat(60));
    println!("Processing Summary:");
    println!("  Total files processed: {}", completed_count);
    println!("  Files converted:       {}", converted_count);
    println!("  Files copied:          {}", copied_count);
    println!("  Files skipped:         {}", skipped_count);
    println!("  Files with errors:     {}", error_count);
    println!(
        "  Total original size (converted files): {}",
        human_bytes::human_bytes(total_original_size as f64)
    );
    println!(
        "  Total converted size (converted files): {}",
        human_bytes::human_bytes(total_converted_size as f64)
    );

    let total_saved_size = total_original_size.saturating_sub(total_converted_size);
    println!(
        "  Total storage saved (converted files): {}",
        human_bytes::human_bytes(total_saved_size as f64)
    );
    println!("{}", "-".repeat(60));

    Ok(())
}
