# Status App Progress

## Completed ✅

### Core Functionality
- Forked from Bailey Townsend's Rusty Statusphere
- Multi-user support with BlueSky OAuth authentication
- Custom lexicon: `io.zzstoatzz.status.record` with emoji + optional text
- Status expiration times (30min to 1 week)
- Real-time updates via Jetstream firehose
- Public profiles at status.zzstoatzz.io/@handle
- Global feed showing all statuses
- Database persistence on Fly.io

### OAuth Implementation
- Fixed OAuth callback error handling (missing 'code' field)
- Reverted to working state with `transition:generic` scope
- Research complete: No granular permissions available yet in AT Protocol
- Must use broad permissions until Auth Scopes feature ships

### UI/UX
- One-time walkthrough for new users (stored in localStorage)
- Fixed double @ symbol in feed username links
- Fixed feed ordering (newest first by startedAt)
- Emoji picker with visual selection
- Status expiration display with relative times

## Current State 🚧
- App deployed and functional at status.zzstoatzz.io
- OAuth works but requires broad permissions (AT Protocol limitation)
- All core features operational

## Today's Progress (Sept 1, 2025)
- Forked from Bailey's emoji-only statusphere
- Created custom lexicon with text + expiration support
- Added multi-user OAuth authentication
- Implemented emoji picker with keyword search
- Fixed mobile responsiveness
- Added status expiration (30min to 1 week)
- Set up CI/CD with GitHub Actions
- Renamed repo to "status"
- Improved delete UX (removed confusing clear button)
- Made feed handles visually distinct
- Updated link previews to be lowercase and include actual status
- Cleaned up dead code from original fork
- Posted thread about the launch

## Progress Update (Sept 2, 2025)

### Major Features Added
- **Custom Emoji Support**: Integrated 1600+ animated emojis from bufo.zone
  - Scraped and stored in `/static/emojis/`
  - Searchable in emoji picker
  - Supports GIF animation
  - No database needed - served directly from filesystem
- **Infinite Scrolling**: Global feed now loads forever
  - Added `/api/feed` endpoint with pagination
  - Smooth loading with "beginning of time" indicator
  - Handles large datasets efficiently
- **Theme Consistency**: Added theme toggle indicator across all pages
- **Performance Optimization**: Added database indexes on critical columns
  - `idx_status_startedAt` for feed queries
  - `idx_status_authorDid_startedAt` for user queries

### Bug Fixes
- Fixed favicon not loading in production
- Fixed custom emoji layout issues in picker
- Fixed theme toggle icons being invisible
- Removed unused CSS file and public directory
- Suppressed dead_code warning for auto-generated lexicons

### Code Quality Improvements
- Created 5 GitHub issues for technical debt:
  - ✅ #1: Database indexes (COMPLETED)
  - #2: Excessive unwrap() usage (57 instances)
  - #3: Duplicated handle resolution code
  - #4: Hardcoded configuration values
  - #5: No rate limiting on API endpoints
- Cleaned up unused `public/css` directory
- Removed hardcoded OWNER_DID references

## Next Steps 📋

### Immediate
1. **Persistent Session Storage**: Users currently must re-login each visit
2. **UI Polish**: Small visual improvements needed

### Location Feature (Proposed)
- Add optional location to statuses
- Browser geolocation API integration
- Privacy controls (location blurring)
- Future: Integrate with SmokeSignal's location standards
- Vision: Global map of statuses

### Future Considerations
- Migrate to granular OAuth scopes when available
- H3 hexagon location support
- SmokeSignal event integration
- Location-based discovery

## Progress Update (Sept 2, 2025 - Evening)

### Testing Infrastructure & Resilience
- **Test Framework Setup**: Established comprehensive testing with `just test` command
  - 9 tests covering rate limiting, error handling, and API endpoints
  - All tests passing
- **Rate Limiting**: Implemented token bucket algorithm
  - 30 requests per minute per IP address on `/status` endpoint
  - Prevents spam and abuse
  - Closes GitHub issue #5
- **Error Handling**: Centralized error handling with `AppError` enum
  - Consistent error responses across the application
  - Better debugging and user feedback

### Admin Moderation System
- **Soft Hide Capability**: Added ability to hide inappropriate content
  - Posts remain in database but excluded from global feed
  - Admin DID hardcoded: `did:plc:xbtmt2zjwlrfegqvch7fboei` (zzstoatzz.io)
  - `/admin/hide-status` endpoint for toggling visibility
  - Hide button in UI visible only to admin
  - Confirmation dialog before hiding

### UI Improvements
- **Fixed Emoji Alignment**: Resolved custom emoji sizing issues in status history
  - Standardized container dimensions (1.5rem x 1.5rem for history items)
  - Consistent layout regardless of emoji type

### DevOps & CI/CD
- **Review Apps**: Set up automatic preview deployments for PRs
  - Uses GitHub Actions with `superfly/fly-pr-review-apps@1.2.1`
  - Deploys to `pr-<number>-zzstoatzz-status.fly.dev`
  - Smaller resources for review apps (256MB RAM)
  - Updated FLY_API_TOKEN to org-level token for app creation

### Code Quality
- **Refactoring**: Cleaned up parameter passing
  - Replaced verbose `&dyn rusqlite::ToSql` with `rusqlite::params!` macro
  - More idiomatic Rust code

## Technical Debt
- ✅ ~~No rate limiting on API endpoints~~ (RESOLVED with issue #5)
- OAuth scopes too broad (waiting on AT Protocol)
- Session persistence needed
- Location feature architecture planned but not implemented
- #2: Excessive unwrap() usage (57 instances)
- #3: Duplicated handle resolution code
- #4: Hardcoded configuration values

## Resources
- OAuth research: `/tmp/atproto-oauth-research/`
- Location proposal: `/tmp/atproto-oauth-research/location_integration_proposal.md`
- PR #7: Testing, rate limiting, and moderation features