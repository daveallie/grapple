# Change Log

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

Fixes:

- Reduce high CPU usage when using `--thread-bandwidth`

## [0.3.0] - 2018-09-08

New features:

- Support for Basic/Digest auth username and password to be set via CLI options
- Per-thread bandwidth limit option

## [0.2.2] - 2017-04-08

New features:

- Global progress bar to see progress of download as a whole

Fixes:

- Don't mark the file as complete if some parts failed to download
- If last part was already downloaded when resuming, request was made out of file bounds

## [0.2.1] - 2017-03-26

Fixes:

- Data missing from final file if the download was interrupted

## [0.2.0] - 2017-03-26

New features:

- Parts flag - Can now specify number of parts independent of threads

Fixes:

- Moved to github version of `pbr` to fix [a8m/pb#48](https://github.com/a8m/pb/pull/48)

[Unreleased]: https://github.com/daveallie/bindrs/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/daveallie/bindrs/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/daveallie/bindrs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/daveallie/bindrs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/daveallie/bindrs/compare/v0.1.0...v0.2.0
