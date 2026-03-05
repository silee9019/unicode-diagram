package dsl

import "github.com/silee9019/unicode-diagram/internal/object"

type DslCommand interface {
	isDslCommand()
}

type CanvasSize struct {
	IsAuto bool
	Value  int
}

type CanvasCmd struct {
	Width, Height   CanvasSize
	Border          *object.BorderStyle
	ContentOverflow *object.ContentOverflow
	ContentAlign    *object.ContentAlign
}

func (c *CanvasCmd) isDslCommand() {}

type CollisionCmd struct {
	On bool
}

func (c *CollisionCmd) isDslCommand() {}

type ObjectCmd struct {
	Object object.DrawObject
}

func (c *ObjectCmd) isDslCommand() {}

type ArrowCmd struct {
	SrcID, DstID     string
	SrcSide, DstSide object.Side
	Head             rune
	HasHead          bool
	Both             bool
	Legend           *object.Legend
	Line             int
}

func (c *ArrowCmd) isDslCommand() {}

type ArrowheadCmd struct {
	Ch rune
}

func (c *ArrowheadCmd) isDslCommand() {}
