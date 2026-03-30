---
title: "DepotDownloader"
type: domain
tags: [depotdownloader, steam, sidecar, gpl-2, dependencies]
created: 2026-03-30
updated: 2026-03-30
---

# DepotDownloader

[DepotDownloader](https://github.com/SteamRE/DepotDownloader) is an open-source tool for downloading content from Steam's CDN. It is the primary external dependency for Rewind's download functionality.

## What It Does

DepotDownloader authenticates with Steam and downloads depot content by manifest ID. It supports:

- Downloading full depot contents for a specific manifest
- Downloading a subset of files via `-filelist`
- Fetching manifest metadata only via `-manifest-only` (no file downloads)
- Session caching with `-remember-password`

## CLI Interface

Key commands used by Rewind:

### Fetch manifest metadata (no download)

```
DepotDownloader \
  -app <appid> \
  -depot <depotid> \
  -manifest <manifestid> \
  -manifest-only \
  -username <user> \
  -remember-password \
  -dir <output_dir>
```

Produces a text file listing all files with SHA hashes, sizes, and chunk counts. Used for diffing two versions (Step 5 of the downgrade process).

### Download changed files

```
DepotDownloader \
  -app <appid> \
  -depot <depotid> \
  -manifest <manifestid> \
  -username <user> \
  -remember-password \
  -filelist <changed_files.txt> \
  -dir <download_dir>
```

Downloads only the files listed in the filelist. Used after manifest diffing to minimize download size.

## Manifest Output Format

The `-manifest-only` flag produces output like:

```
Content Manifest for Depot 3321461

Manifest ID / date     : 3559081655545104676 / 03/22/2026 16:01:45
Total number of files  : 257
Total number of chunks : 130874
Total bytes on disk    : 133352312992
Total bytes compressed : 100116131120

          Size Chunks File SHA                                 Flags Name
       6740755      7 8a11847b3e22b2fb909b57787ed94d1bb139bcb2     0 0000/0.pamt
     912261088    896 3e6800918fef5f8880cf601e5b60bff031465e60     0 0000/0.paz
```

Rewind parses this format to build file-level diffs between versions.

## Authentication

DepotDownloader requires a valid Steam account. It supports:

- Username/password authentication
- Steam Guard (email and mobile 2FA)
- Session caching via `-remember-password` (stores tokens locally)

Rewind collects credentials in-app and passes them to DepotDownloader as command-line arguments. See [decisions/depotdownloader-sidecar](../decisions/depotdownloader-sidecar.md) for the integration approach.

## Licensing

DepotDownloader is licensed under **GPL-2.0**. This has a direct impact on Rewind's licensing -- see [decisions/gpl2-licensing](../decisions/gpl2-licensing.md).

Key implications:
- Rewind must also be GPL-2.0 (or GPL-2.0-compatible) since it bundles DepotDownloader
- Source code must be made available to users who receive the binary
- Modifications to DepotDownloader (if any) must be shared under the same license

## Limitations

- Requires .NET runtime in its standard form. Rewind avoids this by using the self-contained (ahead-of-time compiled) build -- see [decisions/depotdownloader-sidecar](../decisions/depotdownloader-sidecar.md).
- No programmatic API -- Rewind interacts with it as a subprocess, parsing stdout for progress.
- Download speed depends on Steam's CDN and the user's connection. Large games (50+ GB of changed files) can take hours.
- Error reporting is via exit codes and stderr text, which Rewind must parse and translate into user-friendly messages.

## Future Considerations

- Direct integration with SteamKit2 (the .NET library DepotDownloader is built on) could eliminate the subprocess overhead, but would require either Rust bindings or a reimplementation of the relevant protocols.
