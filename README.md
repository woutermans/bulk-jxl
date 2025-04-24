# Bulk JXL Converter

A command-line tool written in Rust to bulk convert images to the JPEG XL (JXL) format and copy other files, while preserving metadata and file timestamps.

## Features

*   **Bulk Conversion:** Convert multiple image files in a directory to JXL format.
*   **Recursive Processing:** Optionally process files in subdirectories.
*   **Parallel Processing:** Utilize multiple jobs for faster conversion.
*   **Metadata Preservation:** Copies EXIF data and file modification timestamps.
*   **File Copying:** Optionally copy non-image files alongside converted images.
*   **Progress Indication:** Shows progress during processing.
*   **Summary Report:** Provides a summary of processed files, conversion statistics, and errors.

## Prerequisites

*   **Rust and Cargo:** You need to have Rust and Cargo installed. Follow the instructions on the [official Rust website](https://www.rust-lang.org/tools/install).
*   **ffmpeg:** The tool uses `ffmpeg` for image conversion. Make sure `ffmpeg` is installed and available in your system's PATH. You can usually install it via your system's package manager (e.g., `apt`, `brew`, `choco`).

## Building

Navigate to the project directory in your terminal and build the project using Cargo:

```bash
cargo build --release
```

This will create an executable in the `target/release/` directory.

## Usage

Run the executable from the project root directory.

```bash
./target/release/bulk-jxl [OPTIONS]
```

### Options

*   `-i, --input <INPUT>`: **Required.** The input directory containing the images and files to process.
*   `-o, --output <OUTPUT>`: **Required.** The output directory where converted JXL files and copied files will be placed. Directories will be created if they don't exist.
*   `-r, --recursive`: Process files in subdirectories recursively.
*   `-j, --jobs <JOBS>`: The number of parallel jobs to run for processing. Defaults to 2.
*   `-c, --copy-all`: Copy all files from the input directory to the output directory, not just accepted image types.

### Example

Convert all supported images in the `input_images` directory and its subdirectories to JXL, placing the output in `output_jxl`, using 4 parallel jobs:

```bash
./target/release/bulk-jxl -i input_images -o output_jxl -r -j 4
```

Copy all files (images and others) from `source_files` to `destination_backup`:

```bash
./target/release/bulk-jxl -i source_files -o destination_backup -c
```

## Supported Image Extensions

The tool supports converting a wide range of image formats to JXL, leveraging the capabilities of ffmpeg. The currently accepted extensions include:

*   **Joint Photographic Experts Group:** jpg, jpeg, jpe, jif, jfif, jfi
*   **Portable Network Graphics:** png
*   **WebP:** webp
*   **Graphics Interchange Format:** gif
*   **Bitmap:** bmp, dib
*   **Portable Pixmap:** ppm
*   **Portable Graymap:** pgm
*   **Portable Anymap:** pam
*   **Tagged Image File Format:** tif, tiff
*   **Targa Graphics Format:** tga, icb, vda, vst
*   **DirectDraw Surface:** dds
*   **OpenEXR:** exr
*   **High Dynamic Range:** hdr, pic
*   **Microsoft Icon:** ico
*   **Flexible Image Transport System:** fits
*   **Alias/Wavefront PIX image:** pix
*   **BRender PIX image:** brender_pix
*   **JPEG 2000:** j2k, jp2, jpt
*   **JPEG-LS:** jls
*   **Portable Bitmap:** pbm
*   **PC Paintbrush:** pcx
*   **Portable FloatMap:** pfm
*   **Portable Graymap with YUV:** pgmyuv
*   **JPEG 2000 part 1:** pgx
*   **Portable Half FloatMap:** phm
*   **Photo CD:** pcd
*   **Pictor:** pct, pict
*   **Adobe Photoshop:** psd
*   **QuickDraw:** qdraw
*   **Quite OK Image:** qoi
*   **Silicon Graphics Image:** sgi
*   **Sun Raster:** ras
*   **VBN:** vbn
*   **XBM:** xbm
*   **XPM:** xpm
*   **XWD:** xwd

Not all extensions are tested.
