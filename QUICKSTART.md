
  🚀 Running Auto-Dev as a Background Service

  Quick Start Methods:

  1. Simple Background Mode (aliases for convenience):
  # Either of these work - they're aliases
  auto-dev run
  auto-dev start

  2. Initialize Project First (recommended):
  # First, initialize auto-dev in your project
  auto-dev init

  # Then start the background loop
  auto-dev run

  3. Advanced Loop Control:
  # More control over the loop
  auto-dev loop --help

  # Start in background with specific port
  auto-dev loop start --background --port 9090

  # Check status
  auto-dev loop status

  # Stop the loop
  auto-dev loop stop

  # Trigger manual events
  auto-dev loop trigger file-changed src/main.rs

  What It Does:

  When running in background mode, auto-dev:
  - Monitors file changes in your project
  - Responds to events (test failures, spec changes, code modifications)
  - Can automatically generate tests, fix issues, update documentation
  - Runs on port 9090 by default for status checking
  - Creates .auto-dev/ directory for configuration and state

  Configuration:

  After running auto-dev init, you'll get a .auto-dev/config.toml file you can customize:
  - Event handlers
  - File monitoring patterns
  - LLM provider settings
  - Autonomous behavior rules

  Try It Now:

  cd your-project
  auto-dev init
  auto-dev run

  The loop will start monitoring your project and can respond to changes automatically based on your configuration!
  