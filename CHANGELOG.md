# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

## [Unreleased]

## [0.2.2] - 2017-04-08
### Added
- Global progress bar to see progress of download as a whole

### Fixed
- Don't mark the file as complete if some parts failed to download
- If last part was already downloaded when resuming, request was made out of file bounds

## [0.2.1] - 2017-03-26
### Fixed
- Data missing from final file if the download was interrupted

## [0.2.0] - 2017-03-26
### Added
- Parts flag - Can now specify number of parts independent of threads

### Fixed
- Moved to github version of `pbr` to fix https://github.com/a8m/pb/pull/48

[Unreleased]: https://github.com/daveallie/bindrs/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/daveallie/bindrs/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/daveallie/bindrs/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/daveallie/bindrs/compare/v0.1.0...v0.2.0
