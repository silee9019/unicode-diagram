package dsl

import (
	"fmt"
	"strconv"
	"strings"
	"unicode"

	uerr "github.com/silee-tools/unid/internal/errors"
	"github.com/silee-tools/unid/internal/object"
)

func Parse(input string) ([]DslCommand, error) {
	lines := strings.Split(input, "\n")
	var commands []DslCommand

	for i, line := range lines {
		lineNum := i + 1
		trimmed := strings.TrimSpace(line)
		if trimmed == "" || strings.HasPrefix(trimmed, "#") {
			continue
		}
		tokens := strings.Fields(trimmed)
		if len(tokens) == 0 {
			continue
		}
		cmd, err := parseCommand(tokens, lineNum)
		if err != nil {
			return nil, err
		}
		commands = append(commands, cmd)
	}
	return commands, nil
}

func parseCommand(tokens []string, line int) (DslCommand, error) {
	keyword := strings.ToLower(tokens[0])
	switch keyword {
	case "canvas":
		return parseCanvas(tokens, line)
	case "collision":
		return parseCollision(tokens, line)
	case "box", "rect":
		return parseRect(tokens, line)
	case "text":
		return parseText(tokens, line)
	case "hline":
		return parseHLine(tokens, line)
	case "vline":
		return parseVLine(tokens, line)
	case "arrow":
		return parseArrow(tokens, line)
	case "arrowhead":
		return parseArrowhead(tokens, line)
	default:
		return nil, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown command '%s'", tokens[0])}
	}
}

func parseCanvas(tokens []string, line int) (DslCommand, error) {
	if len(tokens) < 2 {
		return nil, &uerr.ParseError{Line: line, Message: "canvas requires size arguments (e.g., 'canvas 40 10' or 'canvas auto')"}
	}

	cmd := &CanvasCmd{}
	var optsStart int

	if strings.ToLower(tokens[1]) == "auto" {
		cmd.Width = CanvasSize{IsAuto: true}
		cmd.Height = CanvasSize{IsAuto: true}
		optsStart = 2
	} else {
		if len(tokens) < 3 {
			return nil, &uerr.ParseError{Line: line, Message: "canvas requires width and height (e.g., 'canvas 40 10')"}
		}
		w, err := parseUint(tokens[1], "canvas width", line)
		if err != nil {
			return nil, err
		}
		h, err := parseUint(tokens[2], "canvas height", line)
		if err != nil {
			return nil, err
		}
		cmd.Width = CanvasSize{Value: w}
		cmd.Height = CanvasSize{Value: h}
		optsStart = 3
	}

	for _, token := range tokens[optsStart:] {
		if v, ok := stripOption(token, "border", "b"); ok {
			bs, err := parseBorderStyle(v, line)
			if err != nil {
				return nil, err
			}
			cmd.Border = &bs
		} else if v, ok := stripOption(token, "overflow", "o"); ok {
			co, err := parseContentOverflow(v, line)
			if err != nil {
				return nil, err
			}
			cmd.ContentOverflow = &co
		} else if v, ok := stripOption(token, "align", "a"); ok {
			ca, err := parseContentAlign(v, line)
			if err != nil {
				return nil, err
			}
			cmd.ContentAlign = &ca
		} else {
			return nil, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown canvas option '%s'", token)}
		}
	}
	return cmd, nil
}

func parseCollision(tokens []string, line int) (DslCommand, error) {
	if len(tokens) < 2 {
		return nil, &uerr.ParseError{Line: line, Message: "collision requires 'on' or 'off'"}
	}
	switch strings.ToLower(tokens[1]) {
	case "on":
		return &CollisionCmd{On: true}, nil
	case "off":
		return &CollisionCmd{On: false}, nil
	default:
		return nil, &uerr.ParseError{Line: line, Message: fmt.Sprintf("collision must be 'on' or 'off', got '%s'", tokens[1])}
	}
}

func parseRect(tokens []string, line int) (DslCommand, error) {
	if len(tokens) < 5 {
		return nil, &uerr.ParseError{Line: line, Message: "box requires col, row, width, height"}
	}
	col, err := parseUint(tokens[1], "col", line)
	if err != nil {
		return nil, err
	}
	row, err := parseUint(tokens[2], "row", line)
	if err != nil {
		return nil, err
	}
	w, err := parseUint(tokens[3], "width", line)
	if err != nil {
		return nil, err
	}
	h, err := parseUint(tokens[4], "height", line)
	if err != nil {
		return nil, err
	}

	rect := object.NewRect(col, row, w, h)
	greedyIdx := greedyTokenIndex(tokens, 5)

	var lgPos *object.LegendPos
	var lgOverflow *object.ContentOverflow
	var lgAlign *object.ContentAlign

	for _, token := range tokens[5:greedyIdx] {
		if v, ok := stripOption(token, "id"); ok {
			if err := validateID(v, line); err != nil {
				return nil, err
			}
			rect.ID = v
		} else if v, ok := stripOption(token, "style", "s"); ok {
			bs, err := parseBorderStyle(v, line)
			if err != nil {
				return nil, err
			}
			rect.Style = bs
		} else if v, ok := stripOption(token, "overflow", "o"); ok {
			co, err := parseContentOverflow(v, line)
			if err != nil {
				return nil, err
			}
			rect.ContentOverflow = co
		} else if v, ok := stripOption(token, "align", "a"); ok {
			ca, err := parseContentAlign(v, line)
			if err != nil {
				return nil, err
			}
			rect.ContentAlign = ca
		} else if v, ok := stripOption(token, "legend-pos", "lp"); ok {
			pos, err := parseLegendPos(v, line)
			if err != nil {
				return nil, err
			}
			if pos == object.LegendLeft || pos == object.LegendRight {
				return nil, &uerr.ParseError{Line: line, Message: "box legend-pos only supports top(t) or bottom(b)"}
			}
			lgPos = &pos
		} else if v, ok := stripOption(token, "legend-overflow", "lo"); ok {
			co, err := parseContentOverflow(v, line)
			if err != nil {
				return nil, err
			}
			lgOverflow = &co
		} else if v, ok := stripOption(token, "legend-align", "la"); ok {
			ca, err := parseContentAlign(v, line)
			if err != nil {
				return nil, err
			}
			lgAlign = &ca
		} else {
			return nil, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown box option '%s'", token)}
		}
	}

	if greedyIdx < len(tokens) {
		for i := greedyIdx; i < len(tokens); i++ {
			if strings.HasPrefix(tokens[i], "content=") || strings.HasPrefix(tokens[i], "c=") {
				content, err := extractContent(tokens, i, line)
				if err != nil {
					return nil, err
				}
				rect.Content = content
				rect.HasContent = true
				break
			}
		}
		for i := greedyIdx; i < len(tokens); i++ {
			if strings.HasPrefix(tokens[i], "legend=") || strings.HasPrefix(tokens[i], "lg=") {
				lgText, err := extractLegend(tokens, i, line)
				if err != nil {
					return nil, err
				}
				pos := object.LegendTop
				if lgPos != nil {
					pos = *lgPos
				}
				overflow := object.OverflowEllipsis
				if lgOverflow != nil {
					overflow = *lgOverflow
				}
				align := object.AlignLeft
				if lgAlign != nil {
					align = *lgAlign
				}
				rect.Legend = &object.Legend{Text: lgText, Pos: pos, Overflow: overflow, Align: align}
				break
			}
		}
	}

	return &ObjectCmd{Object: rect}, nil
}

func parseText(tokens []string, line int) (DslCommand, error) {
	if len(tokens) < 4 {
		return nil, &uerr.ParseError{Line: line, Message: "text requires col, row, content=<value>"}
	}
	col, err := parseUint(tokens[1], "col", line)
	if err != nil {
		return nil, err
	}
	row, err := parseUint(tokens[2], "row", line)
	if err != nil {
		return nil, err
	}

	greedyIdx := greedyTokenIndex(tokens, 3)
	var id string

	for _, token := range tokens[3:greedyIdx] {
		if v, ok := stripOption(token, "id"); ok {
			if err := validateID(v, line); err != nil {
				return nil, err
			}
			id = v
		} else {
			return nil, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown text option '%s'", token)}
		}
	}

	content, err := extractContent(tokens, greedyIdx, line)
	if err != nil {
		return nil, err
	}
	text := object.NewText(col, row, content)
	text.ID = id
	return &ObjectCmd{Object: text}, nil
}

func parseHLine(tokens []string, line int) (DslCommand, error) {
	if len(tokens) < 4 {
		return nil, &uerr.ParseError{Line: line, Message: "hline requires col, row, length"}
	}
	col, err := parseUint(tokens[1], "col", line)
	if err != nil {
		return nil, err
	}
	row, err := parseUint(tokens[2], "row", line)
	if err != nil {
		return nil, err
	}
	length, err := parseUint(tokens[3], "length", line)
	if err != nil {
		return nil, err
	}

	hl := object.NewHLine(col, row, length)
	greedyIdx := greedyTokenIndex(tokens, 4)

	var lgPos *object.LegendPos
	var lgOverflow *object.ContentOverflow
	var lgAlign *object.ContentAlign

	for _, token := range tokens[4:greedyIdx] {
		if v, ok := stripOption(token, "style", "s"); ok {
			ls, err := parseLineStyle(v, line)
			if err != nil {
				return nil, err
			}
			hl.Style = ls
		} else if v, ok := stripOption(token, "id"); ok {
			if err := validateID(v, line); err != nil {
				return nil, err
			}
			hl.ID = v
		} else if v, ok := stripOption(token, "pos", "position"); ok {
			pos, err := parseLegendPos(v, line)
			if err != nil {
				return nil, err
			}
			lgPos = &pos
		} else if v, ok := stripOption(token, "legend-overflow", "lo"); ok {
			co, err := parseContentOverflow(v, line)
			if err != nil {
				return nil, err
			}
			lgOverflow = &co
		} else if v, ok := stripOption(token, "legend-align", "la"); ok {
			ca, err := parseContentAlign(v, line)
			if err != nil {
				return nil, err
			}
			lgAlign = &ca
		} else {
			return nil, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown hline option '%s'", token)}
		}
	}

	if greedyIdx < len(tokens) {
		for i := greedyIdx; i < len(tokens); i++ {
			if strings.HasPrefix(tokens[i], "legend=") || strings.HasPrefix(tokens[i], "lg=") {
				lgText, err := extractLegend(tokens, i, line)
				if err != nil {
					return nil, err
				}
				pos := object.LegendTop
				if lgPos != nil {
					pos = *lgPos
				}
				overflow := object.OverflowEllipsis
				if lgOverflow != nil {
					overflow = *lgOverflow
				}
				align := object.AlignLeft
				if lgAlign != nil {
					align = *lgAlign
				}
				hl.Legend = &object.Legend{Text: lgText, Pos: pos, Overflow: overflow, Align: align}
				break
			}
		}
	}

	return &ObjectCmd{Object: hl}, nil
}

func parseVLine(tokens []string, line int) (DslCommand, error) {
	if len(tokens) < 4 {
		return nil, &uerr.ParseError{Line: line, Message: "vline requires col, row, length"}
	}
	col, err := parseUint(tokens[1], "col", line)
	if err != nil {
		return nil, err
	}
	row, err := parseUint(tokens[2], "row", line)
	if err != nil {
		return nil, err
	}
	length, err := parseUint(tokens[3], "length", line)
	if err != nil {
		return nil, err
	}

	vl := object.NewVLine(col, row, length)
	greedyIdx := greedyTokenIndex(tokens, 4)

	var lgPos *object.LegendPos
	var lgOverflow *object.ContentOverflow
	var lgAlign *object.ContentAlign

	for _, token := range tokens[4:greedyIdx] {
		if v, ok := stripOption(token, "style", "s"); ok {
			ls, err := parseLineStyle(v, line)
			if err != nil {
				return nil, err
			}
			vl.Style = ls
		} else if v, ok := stripOption(token, "id"); ok {
			if err := validateID(v, line); err != nil {
				return nil, err
			}
			vl.ID = v
		} else if v, ok := stripOption(token, "pos", "position"); ok {
			pos, err := parseLegendPos(v, line)
			if err != nil {
				return nil, err
			}
			lgPos = &pos
		} else if v, ok := stripOption(token, "legend-overflow", "lo"); ok {
			co, err := parseContentOverflow(v, line)
			if err != nil {
				return nil, err
			}
			lgOverflow = &co
		} else if v, ok := stripOption(token, "legend-align", "la"); ok {
			ca, err := parseContentAlign(v, line)
			if err != nil {
				return nil, err
			}
			lgAlign = &ca
		} else {
			return nil, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown vline option '%s'", token)}
		}
	}

	if greedyIdx < len(tokens) {
		for i := greedyIdx; i < len(tokens); i++ {
			if strings.HasPrefix(tokens[i], "legend=") || strings.HasPrefix(tokens[i], "lg=") {
				lgText, err := extractLegend(tokens, i, line)
				if err != nil {
					return nil, err
				}
				pos := object.LegendRight
				if lgPos != nil {
					pos = *lgPos
				}
				overflow := object.OverflowEllipsis
				if lgOverflow != nil {
					overflow = *lgOverflow
				}
				align := object.AlignLeft
				if lgAlign != nil {
					align = *lgAlign
				}
				vl.Legend = &object.Legend{Text: lgText, Pos: pos, Overflow: overflow, Align: align}
				break
			}
		}
	}

	return &ObjectCmd{Object: vl}, nil
}

func parseArrow(tokens []string, line int) (DslCommand, error) {
	if len(tokens) < 3 {
		return nil, &uerr.ParseError{Line: line, Message: "arrow requires <src_id>.<side> <dst_id>.<side> (e.g., 'arrow api.right db.top')"}
	}
	srcID, srcSide, err := parseAnchor(tokens[1], line)
	if err != nil {
		return nil, err
	}
	dstID, dstSide, err := parseAnchor(tokens[2], line)
	if err != nil {
		return nil, err
	}

	cmd := &ArrowCmd{SrcID: srcID, SrcSide: srcSide, DstID: dstID, DstSide: dstSide, Line: line}
	greedyIdx := greedyTokenIndex(tokens, 3)

	var lgPos *object.LegendPos
	var lgOverflow *object.ContentOverflow
	var lgAlign *object.ContentAlign

	for _, token := range tokens[3:greedyIdx] {
		if v, ok := stripOption(token, "head"); ok {
			runes := []rune(v)
			if len(runes) == 0 {
				return nil, &uerr.ParseError{Line: line, Message: "head= requires a character value"}
			}
			ch := runes[0]
			if !object.IsValidArrowhead(ch) {
				chars := object.ValidArrowheadChars()
				families := make([]string, 0)
				for i := 0; i < len(chars); i += 4 {
					families = append(families, string(chars[i:i+4]))
				}
				return nil, &uerr.ParseError{Line: line, Message: fmt.Sprintf("invalid arrowhead '%c' (valid families: %s)", ch, strings.Join(families, ", "))}
			}
			cmd.Head = ch
			cmd.HasHead = true
		} else if token == "both" {
			cmd.Both = true
		} else if v, ok := stripOption(token, "pos", "position"); ok {
			pos, err := parseLegendPos(v, line)
			if err != nil {
				return nil, err
			}
			lgPos = &pos
		} else if v, ok := stripOption(token, "legend-overflow", "lo"); ok {
			co, err := parseContentOverflow(v, line)
			if err != nil {
				return nil, err
			}
			lgOverflow = &co
		} else if v, ok := stripOption(token, "legend-align", "la"); ok {
			ca, err := parseContentAlign(v, line)
			if err != nil {
				return nil, err
			}
			lgAlign = &ca
		} else {
			return nil, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown arrow option '%s'", token)}
		}
	}

	if greedyIdx < len(tokens) {
		for i := greedyIdx; i < len(tokens); i++ {
			if strings.HasPrefix(tokens[i], "legend=") || strings.HasPrefix(tokens[i], "lg=") {
				lgText, err := extractLegend(tokens, i, line)
				if err != nil {
					return nil, err
				}
				pos := object.LegendAuto
				if lgPos != nil {
					pos = *lgPos
				}
				overflow := object.OverflowEllipsis
				if lgOverflow != nil {
					overflow = *lgOverflow
				}
				align := object.AlignCenter
				if lgAlign != nil {
					align = *lgAlign
				}
				cmd.Legend = &object.Legend{Text: lgText, Pos: pos, Overflow: overflow, Align: align}
				break
			}
		}
	}

	return cmd, nil
}

func parseArrowhead(tokens []string, line int) (DslCommand, error) {
	if len(tokens) < 2 {
		return nil, &uerr.ParseError{Line: line, Message: "arrowhead requires a character (e.g., 'arrowhead ▶')"}
	}
	runes := []rune(tokens[1])
	if len(runes) == 0 {
		return nil, &uerr.ParseError{Line: line, Message: "arrowhead requires a character value"}
	}
	ch := runes[0]
	if !object.IsValidArrowhead(ch) {
		chars := object.ValidArrowheadChars()
		families := make([]string, 0)
		for i := 0; i < len(chars); i += 4 {
			families = append(families, string(chars[i:i+4]))
		}
		return nil, &uerr.ParseError{Line: line, Message: fmt.Sprintf("invalid arrowhead '%c' (valid families: %s)", ch, strings.Join(families, ", "))}
	}
	return &ArrowheadCmd{Ch: ch}, nil
}

// --- helpers ---

func extractContent(tokens []string, from, line int) (string, error) {
	for i := from; i < len(tokens); i++ {
		var valueStart string
		var found bool
		if after, ok := strings.CutPrefix(tokens[i], "content="); ok {
			valueStart = after
			found = true
		} else if after, ok := strings.CutPrefix(tokens[i], "c="); ok {
			valueStart = after
			found = true
		}
		if found {
			parts := make([]string, 0)
			if valueStart != "" {
				parts = append(parts, valueStart)
			}
			parts = append(parts, tokens[i+1:]...)
			if len(parts) == 0 {
				return "", &uerr.ParseError{Line: line, Message: "content= requires a value"}
			}
			content := strings.Join(parts, " ")
			content = strings.ReplaceAll(content, `\n`, "\n")
			// Trim each line
			lines := strings.Split(content, "\n")
			for j := range lines {
				lines[j] = strings.TrimSpace(lines[j])
			}
			return strings.Join(lines, "\n"), nil
		}
	}
	return "", &uerr.ParseError{Line: line, Message: "missing content= (or c=)"}
}

func extractLegend(tokens []string, from, line int) (string, error) {
	for i := from; i < len(tokens); i++ {
		var valueStart string
		var found bool
		if after, ok := strings.CutPrefix(tokens[i], "legend="); ok {
			valueStart = after
			found = true
		} else if after, ok := strings.CutPrefix(tokens[i], "lg="); ok {
			valueStart = after
			found = true
		}
		if found {
			parts := make([]string, 0)
			if valueStart != "" {
				parts = append(parts, valueStart)
			}
			for _, t := range tokens[i+1:] {
				if strings.HasPrefix(t, "content=") || strings.HasPrefix(t, "c=") {
					break
				}
				parts = append(parts, t)
			}
			if len(parts) == 0 {
				return "", &uerr.ParseError{Line: line, Message: "lg= requires a value"}
			}
			content := strings.Join(parts, " ")
			content = strings.ReplaceAll(content, `\n`, "\n")
			lines := strings.Split(content, "\n")
			for j := range lines {
				lines[j] = strings.TrimSpace(lines[j])
			}
			return strings.Join(lines, "\n"), nil
		}
	}
	return "", &uerr.ParseError{Line: line, Message: "missing lg= (or legend=)"}
}

func greedyTokenIndex(tokens []string, from int) int {
	for i := from; i < len(tokens); i++ {
		if strings.HasPrefix(tokens[i], "content=") ||
			strings.HasPrefix(tokens[i], "c=") ||
			strings.HasPrefix(tokens[i], "lg=") ||
			strings.HasPrefix(tokens[i], "legend=") {
			return i
		}
	}
	return len(tokens)
}

func parseAnchor(s string, line int) (string, object.Side, error) {
	idx := strings.LastIndex(s, ".")
	if idx < 0 {
		return "", 0, &uerr.ParseError{Line: line, Message: fmt.Sprintf("invalid anchor '%s' (expected <id>.<side>, e.g., 'api.right')", s)}
	}
	id := s[:idx]
	sideStr := s[idx+1:]
	if id == "" {
		return "", 0, &uerr.ParseError{Line: line, Message: fmt.Sprintf("anchor '%s' has empty id", s)}
	}
	if err := validateID(id, line); err != nil {
		return "", 0, err
	}
	side, err := parseSide(sideStr, s, line)
	if err != nil {
		return "", 0, err
	}
	return id, side, nil
}

func parseSide(s, anchor string, line int) (object.Side, error) {
	switch strings.ToLower(s) {
	case "top", "t":
		return object.SideTop, nil
	case "right", "r":
		return object.SideRight, nil
	case "bottom", "b":
		return object.SideBottom, nil
	case "left", "l":
		return object.SideLeft, nil
	default:
		return 0, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown side '%s' in anchor '%s' (expected top/t, right/r, bottom/b, left/l)", s, anchor)}
	}
}

func validateID(id string, line int) error {
	if id == "" {
		return &uerr.ParseError{Line: line, Message: "id cannot be empty"}
	}
	for _, c := range id {
		if !unicode.IsLetter(c) && !unicode.IsDigit(c) && c != '_' && c != '-' {
			return &uerr.ParseError{Line: line, Message: fmt.Sprintf("invalid id '%s' (only alphanumeric, '_', '-' allowed)", id)}
		}
	}
	return nil
}

func stripOption(token string, names ...string) (string, bool) {
	for _, name := range names {
		prefix := name + "="
		if strings.HasPrefix(token, prefix) {
			return token[len(prefix):], true
		}
	}
	return "", false
}

func parseBorderStyle(s string, line int) (object.BorderStyle, error) {
	switch strings.ToLower(s) {
	case "light", "l":
		return object.BorderLight, nil
	case "heavy", "h":
		return object.BorderHeavy, nil
	case "double", "d":
		return object.BorderDouble, nil
	case "rounded", "r":
		return object.BorderRounded, nil
	case "none":
		return object.BorderLight, nil
	default:
		return 0, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown border style '%s' (expected light/l, heavy/h, double/d, rounded/r)", s)}
	}
}

func parseLineStyle(s string, line int) (object.LineStyle, error) {
	switch strings.ToLower(s) {
	case "light", "l":
		return object.LineLight, nil
	case "heavy", "h":
		return object.LineHeavy, nil
	case "double", "do":
		return object.LineDouble, nil
	case "dash", "da":
		return object.LineDash, nil
	default:
		return 0, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown line style '%s' (expected light/l, heavy/h, double/do, dash/da)", s)}
	}
}

func parseContentOverflow(s string, line int) (object.ContentOverflow, error) {
	switch strings.ToLower(s) {
	case "ellipsis", "el":
		return object.OverflowEllipsis, nil
	case "overflow", "o":
		return object.OverflowOverflow, nil
	case "hidden", "h":
		return object.OverflowHidden, nil
	case "error", "er":
		return object.OverflowError, nil
	default:
		return 0, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown overflow mode '%s' (expected ellipsis/el, overflow/o, hidden/h, error/er)", s)}
	}
}

func parseLegendPos(s string, line int) (object.LegendPos, error) {
	switch strings.ToLower(s) {
	case "top", "t":
		return object.LegendTop, nil
	case "bottom", "b":
		return object.LegendBottom, nil
	case "left", "l":
		return object.LegendLeft, nil
	case "right", "r":
		return object.LegendRight, nil
	case "auto", "a":
		return object.LegendAuto, nil
	default:
		return 0, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown legend position '%s' (expected top/t, bottom/b, left/l, right/r, auto/a)", s)}
	}
}

func parseContentAlign(s string, line int) (object.ContentAlign, error) {
	switch strings.ToLower(s) {
	case "left", "l":
		return object.AlignLeft, nil
	case "center", "c":
		return object.AlignCenter, nil
	case "right", "r":
		return object.AlignRight, nil
	default:
		return 0, &uerr.ParseError{Line: line, Message: fmt.Sprintf("unknown align '%s' (expected left/l, center/c, right/r)", s)}
	}
}

func parseUint(s, name string, line int) (int, error) {
	n, err := strconv.Atoi(s)
	if err != nil || n < 0 {
		return 0, &uerr.ParseError{Line: line, Message: fmt.Sprintf("invalid %s '%s' (expected a non-negative integer)", name, s)}
	}
	return n, nil
}
