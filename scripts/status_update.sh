#!/bin/bash
# Investor OS — Daily Status Update Script
# Run this daily to update project status

set -e

echo "📊 Investor OS Status Update"
echo "=============================="
echo ""

# Get current sprint
CURRENT_SPRINT=$(cat .current_sprint 2>/dev/null || echo "unknown")
echo "Current Sprint: $CURRENT_SPRINT"

# Run tests
echo ""
echo "🧪 Running tests..."
if cargo test --lib --quiet 2>&1 | grep -q "test result: ok"; then
    LIB_TESTS="PASS"
    echo "✅ Library tests: PASS"
else
    LIB_TESTS="FAIL"
    echo "❌ Library tests: FAIL"
fi

# Check Golden Path
echo ""
echo "🥇 Checking Golden Path..."
GP_PASS=$(cargo test --lib test_treasury 2>&1 | grep -c "test result: ok" || echo "0")
if [ "$GP_PASS" -gt 0 ]; then
    echo "✅ Golden Path: PASSING"
else
    echo "⚠️  Golden Path: NEEDS ATTENTION"
fi

# Count TODOs
echo ""
echo "📝 Counting tasks..."
TODO_COUNT=$(grep -r "TODO\|FIXME" src/ --include="*.rs" 2>/dev/null | wc -l)
echo "Open TODOs: $TODO_COUNT"

# Git stats
echo ""
echo "🌿 Git activity..."
COMMITS_TODAY=$(git log --oneline --since="24 hours ago" 2>/dev/null | wc -l)
echo "Commits today: $COMMITS_TODAY"

# Update status file
DATE=$(date +%Y-%m-%d)
echo ""
echo "📝 Updating status log..."
cat >> docs/STATUS_LOG.md << EOF

## $DATE - Sprint $CURRENT_SPRINT Update
- **Time:** $(date +%H:%M)
- **Lib Tests:** $LIB_TESTS
- **Golden Path:** $GP_PASS modules passing
- **TODOs:** $TODO_COUNT
- **Commits Today:** $COMMITS_TODAY

EOF

echo ""
echo "✅ Status update complete!"
echo "📄 See docs/STATUS_LOG.md for full history"
echo "📊 See docs/current_status.yaml for detailed status"
