package main_test

import (
	"io"
	"os"
	"os/exec"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

var binaryPath string

func TestMain(m *testing.M) {
	// Build binary
	tmp, err := os.CreateTemp("", "unid-test-*")
	if err != nil {
		panic(err)
	}
	tmp.Close()
	binaryPath = tmp.Name()

	cmd := exec.Command("go", "build", "-o", binaryPath, "./cmd/unid")
	if out, err := cmd.CombinedOutput(); err != nil {
		panic("build failed: " + string(out))
	}

	code := m.Run()
	os.Remove(binaryPath)
	os.Exit(code)
}

func runStdin(input string) (stdout, stderr string, ok bool) {
	cmd := exec.Command(binaryPath)
	cmd.Stdin = strings.NewReader(input)
	var outBuf, errBuf strings.Builder
	cmd.Stdout = &outBuf
	cmd.Stderr = &errBuf
	err := cmd.Run()
	return outBuf.String(), errBuf.String(), err == nil
}

func runSubcmd(subcmd, input string) (stdout, stderr string, ok bool) {
	cmd := exec.Command(binaryPath, subcmd)
	cmd.Stdin = strings.NewReader(input)
	var outBuf, errBuf strings.Builder
	cmd.Stdout = &outBuf
	cmd.Stderr = &errBuf
	err := cmd.Run()
	return outBuf.String(), errBuf.String(), err == nil
}

// suppress unused import
var _ = io.Discard

// ─── Render (stdin default) ──────────────────────────────────────────

func TestRenderSimpleRect(t *testing.T) {
	stdout, _, ok := runStdin("canvas 6 3\ncollision off\nbox 0 0 4 1")
	require.True(t, ok)
	assert.Equal(t, "┌────┐\n│    │\n└────┘", strings.TrimSpace(stdout))
}

func TestRenderRectWithContent(t *testing.T) {
	stdout, _, ok := runStdin("canvas 12 3\ncollision off\nbox 0 0 10 1 c=Hello")
	require.True(t, ok)
	assert.Contains(t, stdout, "Hello")
	assert.Contains(t, stdout, "┌")
	assert.Contains(t, stdout, "└")
}

func TestRenderCJKContent(t *testing.T) {
	stdout, _, ok := runStdin("canvas 14 3\ncollision off\nbox 0 0 12 1 c=한글 테스트")
	require.True(t, ok)
	assert.Contains(t, stdout, "한글 테스트")
}

func TestRenderAutoCanvas(t *testing.T) {
	stdout, _, ok := runStdin("canvas auto\ncollision off\nbox 0 0 4 1")
	require.True(t, ok)
	assert.Equal(t, "┌────┐\n│    │\n└────┘", strings.TrimSpace(stdout))
}

func TestRenderMultipleStyles(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 12\ncollision off\nbox 0 0 6 1 s=l\nbox 0 3 6 1 s=h\nbox 0 6 6 1 s=d\nbox 0 9 6 1 s=r")
	require.True(t, ok)
	assert.Contains(t, stdout, "┌") // light
	assert.Contains(t, stdout, "┏") // heavy
	assert.Contains(t, stdout, "╔") // double
	assert.Contains(t, stdout, "╭") // rounded
}

func TestRenderAnchorArrowHorizontal(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 5\ncollision off\nbox 0 0 6 1 id=a c=A\nbox 18 0 6 1 id=b c=B\narrow a.r b.l")
	require.True(t, ok)
	assert.Contains(t, stdout, "▶")
	assert.Contains(t, stdout, "─")
}

func TestRenderAnchorArrowVertical(t *testing.T) {
	stdout, _, ok := runStdin("canvas 10 10\ncollision off\nbox 0 0 6 1 id=a c=A\nbox 0 6 6 1 id=b c=B\narrow a.b b.t")
	require.True(t, ok)
	assert.Contains(t, stdout, "▼")
	assert.Contains(t, stdout, "│")
}

func TestRenderAnchorArrowLShape(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 10\ncollision off\nbox 0 0 6 1 id=a c=A\nbox 18 6 6 1 id=b c=B\narrow a.r b.t")
	require.True(t, ok)
	assert.True(t, strings.ContainsAny(stdout, "▶▼"))
	assert.True(t, strings.ContainsAny(stdout, "─│"))
}

func TestRenderAnchorArrowUShape(t *testing.T) {
	stdout, _, ok := runStdin("canvas 20 10\ncollision off\nbox 0 0 6 1 id=a c=A\nbox 0 6 6 1 id=b c=B\narrow a.r b.r")
	require.True(t, ok)
	assert.True(t, strings.ContainsRune(stdout, '┐') || strings.ContainsRune(stdout, '┘'))
}

func TestRenderLines(t *testing.T) {
	stdout, _, ok := runStdin("canvas 10 5\ncollision off\nhline 0 0 5\nvline 0 1 4")
	require.True(t, ok)
	assert.Contains(t, stdout, "─")
	assert.Contains(t, stdout, "│")
}

func TestRenderCJKMixedDiagram(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 10\ncollision off\nbox 0 0 12 1 id=srv c=서버\nbox 18 0 8 1 id=db c=DB\narrow srv.r db.l")
	require.True(t, ok)
	assert.Contains(t, stdout, "서버")
	assert.Contains(t, stdout, "DB")
	assert.Contains(t, stdout, "▶")
}

// ─── Collision ───────────────────────────────────────────────────────

func TestCollisionOnError(t *testing.T) {
	_, stderr, ok := runStdin("canvas 20 5\ncollision on\nbox 0 0 5 1\nbox 3 0 5 1")
	assert.False(t, ok)
	assert.Contains(t, stderr, "collision")
}

func TestCollisionOffAllowsOverlap(t *testing.T) {
	_, _, ok := runStdin("canvas 20 5\ncollision off\nbox 0 0 5 1\nbox 3 0 5 1")
	assert.True(t, ok)
}

func TestCollisionErrorFormat(t *testing.T) {
	_, stderr, ok := runStdin("canvas 20 5\ncollision on\nbox 0 0 5 1\nbox 3 0 5 1")
	assert.False(t, ok)
	assert.Contains(t, stderr, "object #2")
	assert.Contains(t, stderr, "object #1")
	assert.Contains(t, stderr, "overlaps")
	assert.Contains(t, stderr, "size")
}

// ─── Content Overflow ────────────────────────────────────────────────

func TestOverflowEllipsis(t *testing.T) {
	stdout, _, ok := runStdin("canvas 10 3\ncollision off\nbox 0 0 4 1 c=VeryLongText")
	require.True(t, ok)
	assert.Contains(t, stdout, "..12")
}

func TestOverflowHidden(t *testing.T) {
	stdout, _, ok := runStdin("canvas 10 3\ncollision off\nbox 0 0 5 1 overflow=hidden c=HelloWorld")
	require.True(t, ok)
	assert.Contains(t, stdout, "Hello")
	assert.NotContains(t, stdout, "World")
}

func TestOverflowError(t *testing.T) {
	_, stderr, ok := runStdin("canvas 10 3\ncollision off\nbox 0 0 3 1 overflow=error c=VeryLong")
	assert.False(t, ok)
	assert.Contains(t, stderr, "overflow")
}

// ─── Content Alignment ──────────────────────────────────────────────

func TestAlignCenter(t *testing.T) {
	stdout, _, ok := runStdin("canvas 12 3\ncollision off\nbox 0 0 10 1 align=c c=Hi")
	require.True(t, ok)
	assert.Contains(t, stdout, "│    Hi    │")
}

func TestAlignRight(t *testing.T) {
	stdout, _, ok := runStdin("canvas 12 3\ncollision off\nbox 0 0 10 1 align=r c=Hi")
	require.True(t, ok)
	assert.Contains(t, stdout, "│        Hi│")
}

// ─── Canvas Border ──────────────────────────────────────────────────

func TestCanvasBorderRounded(t *testing.T) {
	stdout, _, ok := runStdin("canvas 10 3 border=r\ncollision off")
	require.True(t, ok)
	assert.Contains(t, stdout, "╭")
	assert.Contains(t, stdout, "╯")
}

// ─── List ────────────────────────────────────────────────────────────

func TestListSubcommand(t *testing.T) {
	stdout, _, ok := runSubcmd("list", "canvas 30 5\ncollision on\nbox 0 0 8 1 c=Box\ntext 15 1 c=Hi")
	require.True(t, ok)
	assert.Contains(t, stdout, "Canvas: 30x5")
	assert.Contains(t, stdout, "Collision: on")
	assert.Contains(t, stdout, "Objects: 2")
	assert.Contains(t, stdout, "box")
	assert.Contains(t, stdout, "text")
}

func TestListAutoCanvas(t *testing.T) {
	stdout, _, ok := runSubcmd("list", "canvas auto\ncollision off\nbox 0 0 4 1")
	require.True(t, ok)
	assert.Contains(t, stdout, "(auto)")
}

// ─── Lint ────────────────────────────────────────────────────────────

func TestLintOk(t *testing.T) {
	stdout, _, ok := runSubcmd("lint", "canvas 10 3\ncollision off\nbox 0 0 4 1")
	require.True(t, ok)
	assert.Contains(t, stdout, "OK")
}

func TestLintCollisionError(t *testing.T) {
	stdout, _, ok := runSubcmd("lint", "canvas 10 5\ncollision on\nbox 0 0 5 1\nbox 3 0 5 1")
	assert.False(t, ok)
	assert.Contains(t, stdout, "Errors:")
	assert.Contains(t, stdout, "collision")
}

// ─── Guide ──────────────────────────────────────────────────────────

func TestGuideSubcommand(t *testing.T) {
	cmd := exec.Command(binaryPath, "guide")
	out, err := cmd.Output()
	require.NoError(t, err)
	stdout := string(out)
	assert.Contains(t, stdout, "USAGE:")
	assert.Contains(t, stdout, "DSL SYNTAX:")
	assert.Contains(t, stdout, "BORDER STYLES")
}

// ─── Error cases ────────────────────────────────────────────────────

func TestErrorMissingCanvas(t *testing.T) {
	_, stderr, ok := runStdin("collision on\nbox 0 0 4 1")
	assert.False(t, ok)
	assert.Contains(t, stderr, "canvas")
}

func TestErrorMissingCollision(t *testing.T) {
	_, stderr, ok := runStdin("canvas 10 5\nbox 0 0 4 1")
	assert.False(t, ok)
	assert.Contains(t, stderr, "collision")
}

func TestErrorParseError(t *testing.T) {
	_, stderr, ok := runStdin("canvas 10 5\ncollision on\nbadcmd 1 2")
	assert.False(t, ok)
	assert.Contains(t, stderr, "unknown command")
}

func TestErrorUnknownArrowID(t *testing.T) {
	_, stderr, ok := runStdin("canvas 20 5\ncollision off\nbox 0 0 4 1 id=a\narrow a.r nonexistent.l")
	assert.False(t, ok)
	assert.Contains(t, stderr, "unknown object id")
}

func TestErrorInvalidArrowAnchor(t *testing.T) {
	_, stderr, ok := runStdin("canvas 20 5\ncollision off\narrow noid db.top")
	assert.False(t, ok)
	assert.Contains(t, stderr, "invalid anchor")
}

// ─── Comments and blank lines ───────────────────────────────────────

func TestCommentsAndBlankLines(t *testing.T) {
	stdout, _, ok := runStdin("# This is a comment\ncanvas 6 3\n\ncollision off\n# Another comment\nbox 0 0 4 1")
	require.True(t, ok)
	assert.Equal(t, "┌────┐\n│    │\n└────┘", strings.TrimSpace(stdout))
}

// ─── Text object ────────────────────────────────────────────────────

func TestRenderTextObject(t *testing.T) {
	stdout, _, ok := runStdin("canvas 20 3\ncollision off\ntext 0 0 c=Hello World")
	require.True(t, ok)
	assert.Contains(t, stdout, "Hello World")
}

// ─── Backward compatibility ─────────────────────────────────────────

func TestRectAliasForBox(t *testing.T) {
	stdout, _, ok := runStdin("canvas 6 3\ncollision off\nrect 0 0 4 1")
	require.True(t, ok)
	assert.Equal(t, "┌────┐\n│    │\n└────┘", strings.TrimSpace(stdout))
}

// ─── Shorthand options ──────────────────────────────────────────────

func TestShorthandStyle(t *testing.T) {
	stdout, _, ok := runStdin("canvas 8 3\ncollision off\nbox 0 0 6 1 style=rounded")
	require.True(t, ok)
	assert.Contains(t, stdout, "╭")
}

func TestContentWithNewlineEscape(t *testing.T) {
	stdout, _, ok := runStdin("canvas 12 5\ncollision off\nbox 0 0 10 3 c=Line1\\nLine2")
	require.True(t, ok)
	assert.Contains(t, stdout, "Line1")
	assert.Contains(t, stdout, "Line2")
}

func TestMultilineRectVerticalCenter(t *testing.T) {
	stdout, _, ok := runStdin("canvas 10 5\ncollision off\nbox 0 0 8 3 align=c c=AA\\nBB")
	require.True(t, ok)
	assert.Contains(t, stdout, "AA")
	assert.Contains(t, stdout, "BB")
}

func TestMultilineTextObject(t *testing.T) {
	stdout, _, ok := runStdin("canvas 10 3\ncollision off\ntext 0 0 c=Hello\\nWorld")
	require.True(t, ok)
	assert.Contains(t, stdout, "Hello")
	assert.Contains(t, stdout, "World")
}

// ─── Legend ─────────────────────────────────────────────────────────

func TestRectLegendTop(t *testing.T) {
	stdout, _, ok := runStdin("canvas 14 4\ncollision off\nbox 0 1 10 1 lg=Title")
	require.True(t, ok)
	assert.Contains(t, stdout, "Title")
	assert.Contains(t, stdout, "┌")
}

func TestRectLegendBottom(t *testing.T) {
	stdout, _, ok := runStdin("canvas 14 5\ncollision off\nbox 0 0 10 1 legend-pos=b lg=Footer")
	require.True(t, ok)
	assert.Contains(t, stdout, "Footer")
}

func TestRectLegendLRError(t *testing.T) {
	_, stderr, ok := runStdin("canvas 20 5\ncollision off\nbox 0 0 10 1 legend-pos=l lg=Bad")
	assert.False(t, ok)
	assert.Contains(t, stderr, "legend-pos only supports top")
}

func TestRectContentAndLegend(t *testing.T) {
	stdout, _, ok := runStdin("canvas 14 4\ncollision off\nbox 0 1 10 1 c=Content lg=Title")
	require.True(t, ok)
	assert.Contains(t, stdout, "Content")
	assert.Contains(t, stdout, "Title")
}

func TestHLineLegendTop(t *testing.T) {
	stdout, _, ok := runStdin("canvas 15 3\ncollision off\nhline 0 1 10 lg=separator")
	require.True(t, ok)
	assert.Contains(t, stdout, "separator")
	assert.Contains(t, stdout, "─")
}

func TestVLineLegendRight(t *testing.T) {
	stdout, _, ok := runStdin("canvas 15 5\ncollision off\nvline 0 0 4 lg=axis")
	require.True(t, ok)
	assert.Contains(t, stdout, "axis")
	assert.Contains(t, stdout, "│")
}

func TestHLineWithID(t *testing.T) {
	stdout, _, ok := runStdin("canvas 15 3\ncollision off\nhline 0 1 10 id=sep")
	require.True(t, ok)
	assert.Contains(t, stdout, "─")
}

// ─── Arrow from non-rect objects ────────────────────────────────────

func TestArrowFromHLine(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 5\ncollision off\nhline 0 2 10 id=sep\nbox 18 0 6 1 id=b c=B\narrow sep.r b.l")
	require.True(t, ok)
	assert.True(t, strings.ContainsAny(stdout, "→─"))
}

func TestArrowFromText(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 5\ncollision off\ntext 0 1 id=lbl c=Label\nbox 18 0 6 1 id=b c=B\narrow lbl.r b.l")
	require.True(t, ok)
	assert.True(t, strings.ContainsAny(stdout, "→─"))
}

func TestArrowFromVLine(t *testing.T) {
	stdout, _, ok := runStdin("canvas 20 10\ncollision off\nvline 0 0 5 id=axis\nbox 10 6 6 1 id=b c=B\narrow axis.b b.t")
	require.True(t, ok)
	assert.True(t, strings.ContainsAny(stdout, "↓│"))
}

// ─── Rect ID ────────────────────────────────────────────────────────

func TestRectWithID(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 5\ncollision off\nbox 0 0 8 1 id=mybox c=Hello")
	require.True(t, ok)
	assert.Contains(t, stdout, "Hello")
}

func TestDuplicateIDError(t *testing.T) {
	_, stderr, ok := runStdin("canvas 20 5\ncollision off\nbox 0 0 4 1 id=a\nbox 10 0 4 1 id=a")
	assert.False(t, ok)
	assert.Contains(t, stderr, "duplicate")
}

// ─── Arrowhead + Bidirectional ──────────────────────────────────────

func TestArrowCustomHead(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 3\ncollision off\nbox 0 0 4 1 id=a c=A\nbox 20 0 4 1 id=b c=B\narrow a.r b.l head=▶")
	require.True(t, ok)
	assert.Contains(t, stdout, "▶")
	assert.NotContains(t, stdout, "→")
}

func TestArrowGlobalArrowhead(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 3\ncollision off\narrowhead ▶\nbox 0 0 4 1 id=a c=A\nbox 20 0 4 1 id=b c=B\narrow a.r b.l")
	require.True(t, ok)
	assert.Contains(t, stdout, "▶")
}

func TestArrowPerArrowOverridesGlobal(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 3\ncollision off\narrowhead ▶\nbox 0 0 4 1 id=a c=A\nbox 20 0 4 1 id=b c=B\narrow a.r b.l head=⇒")
	require.True(t, ok)
	assert.Contains(t, stdout, "⇒")
	assert.NotContains(t, stdout, "▶")
}

func TestArrowBidirectional(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 3\ncollision off\nbox 0 0 4 1 id=a c=A\nbox 20 0 4 1 id=b c=B\narrow a.r b.l both")
	require.True(t, ok)
	assert.Contains(t, stdout, "◀")
}

func TestArrowBidirectionalWithCustomHead(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 3\ncollision off\nbox 0 0 4 1 id=a c=A\nbox 20 0 4 1 id=b c=B\narrow a.r b.l both head=▶")
	require.True(t, ok)
	assert.Contains(t, stdout, "▶")
	assert.Contains(t, stdout, "◀")
}

func TestArrowInvalidArrowheadRejected(t *testing.T) {
	_, stderr, ok := runStdin("canvas 30 3\ncollision off\nbox 0 0 4 1 id=a c=A\nbox 20 0 4 1 id=b c=B\narrow a.r b.l head=◆")
	assert.False(t, ok)
	assert.Contains(t, stderr, "invalid arrowhead")
}

func TestArrowHeadResolvesDirectionVertical(t *testing.T) {
	stdout, _, ok := runStdin("canvas 20 12\ncollision off\nbox 2 0 6 1 id=a c=A\nbox 2 8 6 1 id=b c=B\narrow a.b b.t head=▶")
	require.True(t, ok)
	assert.Contains(t, stdout, "▼")
}

// ─── Arrow Legend ───────────────────────────────────────────────────

func TestArrowLegendHorizontal(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 5\ncollision off\nbox 0 1 4 1 id=a c=A\nbox 20 1 4 1 id=b c=B\narrow a.r b.l lg=request")
	require.True(t, ok)
	assert.Contains(t, stdout, "request")
}

func TestArrowLegendWithPos(t *testing.T) {
	stdout, _, ok := runStdin("canvas 30 5\ncollision off\nbox 0 1 4 1 id=a c=A\nbox 20 1 4 1 id=b c=B\narrow a.r b.l pos=b lg=data")
	require.True(t, ok)
	assert.Contains(t, stdout, "data")
}

func TestTextOverwritesStructure(t *testing.T) {
	stdout, _, ok := runStdin("canvas 10 3\ncollision off\nbox 0 0 8 1\ntext 0 0 c=X")
	require.True(t, ok)
	assert.True(t, strings.HasPrefix(strings.TrimSpace(stdout), "X"))
}

func TestSelfLoopRightToTop(t *testing.T) {
	stdout, _, ok := runStdin("canvas 20 8\ncollision off\nbox 2 2 8 1 id=a c=Loop\narrow a.r a.t")
	require.True(t, ok)
	assert.Contains(t, stdout, "▼")
	assert.Contains(t, stdout, "┘")
	assert.Contains(t, stdout, "┐")
}

func TestSelfLoopBottomToLeft(t *testing.T) {
	stdout, _, ok := runStdin("canvas 20 8\ncollision off\nbox 2 2 8 1 id=a c=Loop\narrow a.b a.l")
	require.True(t, ok)
	assert.Contains(t, stdout, "▶")
	assert.Contains(t, stdout, "└")
}

func TestArrowLegendVertical(t *testing.T) {
	stdout, _, ok := runStdin("canvas 20 12\ncollision off\nbox 2 0 6 1 id=a c=A\nbox 2 8 6 1 id=b c=B\narrow a.b b.t lg=flow")
	require.True(t, ok)
	assert.Contains(t, stdout, "flow")
}
