# OhPDF Roadmap

> A lightweight, local, single-binary PDF workstation.

OhPDF aims to provide a simple and powerful PDF toolkit for everyone.

OhPDF intentionally avoids becoming an AI PDF reader. It focuses on local PDF operations, while AI capabilities may be integrated later through SDK/plugins.

Unlike traditional PDF software, OhPDF focuses on:

* **Local-first** — files never leave your computer
* **Single binary** — download and run, no installation complexity
* **Lightweight** — keep the application small and fast
* **Simple workflow** — do one PDF task quickly without unnecessary features

OhPDF is not designed to replace professional document suites.
It is designed to be the easiest way to handle everyday PDF operations.

---

# Vision

```
OhPDF

PDF Utility
    +
Automation
    +
Extensibility
```

The goal is to build:

* a lightweight PDF workstation for normal users
* a reliable PDF processing core for developers
* an extensible local PDF ecosystem

---

# Current Version

## v0.1.0 — Core PDF Toolkit

Status: Completed

The first version focuses on essential PDF operations.

### Supported Features

* Image → PDF
* PDF watermark
* PDF compression
* PDF rotation
* PDF → Image
* PDF page sorting
* Extract PDF pages
* PDF encryption
* Merge PDFs
* Split PDFs

### Design Goals

* Single executable
* Local processing only
* No account required
* No file history tracking
* Minimal UI
* Fast startup

---

# Roadmap

---

# v0.1.x — Foundation

## Goal

Make OhPDF stable and prepare the internal architecture.

### Planned

* Improve error handling
* Standardize exit codes
* Improve conversion reports
* Improve core module structure
* Prepare reusable PDF processing APIs

Architecture target:

```
OhPDF
│
├── ohpdf-core
│
├── ohpdf-desktop
│
└── ohpdf-cli
```

---

# v0.2.0 — Automation

## Goal

Make PDF operations faster for power users.

### Features

## Right-click Integration

Allow users to operate PDFs directly from the file manager.

Example:

```
paper.pdf

Right Click

OhPDF
 ├── Compress
 ├── Convert
 ├── Merge
 └── Split
```

---

## CLI Version

Provide a command-line interface.

Examples:

```bash
ohpdf compress input.pdf output.pdf

ohpdf merge a.pdf b.pdf -o result.pdf

ohpdf split input.pdf --pages 1-5
```

Support pipeline-style workflows:

```bash
ohpdf input.pdf \
 | rotate 90 \
 | compress \
 | encrypt password
```

---

## Batch Processing

Support large-scale PDF operations.

Example:

```
📁 papers/

paper1.pdf
paper2.pdf
paper3.pdf

        ↓

Batch Compress

        ↓

📁 output/
```

---

## Drag & Drop Workflow

Reduce unnecessary steps.

Before:

```
Open function
 ↓
Select file
 ↓
Process
```

After:

```
Drag PDF

 ↓

Drop into operation card

 ↓

Done
```

---

# v0.3.0 — Developer Ecosystem

## Goal

Make OhPDF usable as a PDF processing engine.

---

## OhPDF Core API

Internal modules:

```
ohpdf-core

├── merge
├── split
├── compress
├── encrypt
├── convert
└── metadata
```

---

## SDK

Provide language bindings.

### Python

```python
import ohpdf

ohpdf.compress(
    "input.pdf",
    "output.pdf"
)
```

### Node.js

```javascript
await ohpdf.merge([
  "a.pdf",
  "b.pdf"
])
```

---

## Agent Integration

Allow external automation systems to use OhPDF.

Example:

```
Agent

 |

OhPDF SDK

 |

PDF Operations
```

AI is not part of the core product.

Instead, OhPDF provides reliable PDF capabilities that AI systems can call.

---

# v0.4.0 — Power Tools

## Goal

Add professional PDF utilities while keeping the application lightweight.

---

## PDF Inspector

View and modify PDF information.

Features:

* PDF version
* Page information
* File size analysis
* Metadata viewer
* Security information
* Font information

---

## PDF Cleanup

Remove unnecessary or sensitive information.

Features:

* Remove metadata
* Remove hidden information
* Clean annotations
* Remove embedded objects

---

## PDF Diff

Compare two PDF files.

Useful for:

* Contracts
* Academic papers
* Document revisions

Example:

```
version1.pdf

        ↓

      Compare

        ↓

version2.pdf
```

---

## PDF Repair

Recover damaged or incorrectly generated PDF files.

---

## PDF Outline Management

Manage PDF bookmarks and table of contents.

Features:

* Create bookmarks
* Fix incorrect outlines
* Import/export outline structure

---

# v0.5.0 — Productivity

## Goal

Improve daily usage efficiency.

### Features

* Global shortcuts
* Command palette
* Faster navigation
* Improved keyboard workflow

Example:

```
Ctrl + K

compress

↓

Execute PDF compression
```

---

# v0.6.0 — Advanced Features

## Goal

Expand OhPDF capabilities without increasing the core size.

Possible features:

* Advanced PDF analysis
* More conversion tools
* Professional document workflows
* Additional security tools
* Enterprise-oriented features

The main application should still maintain:

```
Single Binary

< 20MB

Local Processing
```

---

# v0.7.0 — Plugin System

## Goal

Allow optional extensions without increasing the core application size.

Architecture:

```
OhPDF

│

├── Core

│

├── Desktop

│

├── CLI

│

└── Plugins
```

Possible plugins:

* OCR
* AI tools
* Office conversion
* Advanced formats
* Translation
* Language packs
* Enterprise extensions

Large features should remain optional.

---

# Long-term Architecture

Final structure:

```
OhPDF

│
├── ohpdf-core
│     ├── merge
│     ├── split
│     ├── compress
│     ├── encrypt
│     └── convert
│
├── ohpdf-desktop
│
├── ohpdf-cli
│
├── bindings
│     ├── python
│     └── node
│
└── plugins
```

---

# Release Philosophy

OhPDF will prioritize:

1. Simplicity over feature count
2. Local processing over cloud dependency
3. Small size over bundled features
4. Stable APIs over fast changes

The core application should always remain:

```
Small.
Fast.
Private.
Simple.
```

---

# License

Planned License:

**Blue Oak Model License 1.0.0**

Reason:

* Open source friendly
* Commercially usable
* Clear legal terms
* Suitable for ecosystem development