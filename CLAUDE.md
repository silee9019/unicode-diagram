# unicode-diagram

Unicode box-drawing 다이어그램 렌더링 CLI. 바이너리 이름: `unid`

## 프로젝트 구조

```
cmd/unid/      CLI 진입점 (cobra), guide 출력
internal/
  errors/      커스텀 에러 타입
  width/       Unicode 너비 유틸리티 (go-runewidth 래핑)
  canvas/      2D 셀 그리드 (display-column 좌표계)
  object/      DrawObject (box, text, hline, vline, arrow)
  dsl/         DSL 텍스트 파서 → DslCommand
  renderer/    Canvas에 DrawObject를 그리는 2-pass 엔진
integration_test.go  CLI 통합 테스트
```

## 개발

- Language: Go 1.23+
- Module: github.com/silee9019/unicode-diagram
- Task Runner: mise
- Build: `mise run build`
- Test: `mise run test`
- Lint: `mise run lint`
- Format: `mise run fmt`

## Guide 예제 관리

- `printGuide()` 예제 DSL 변경 시 OUTPUT 영역을 실제 `go run ./cmd/unid` 결과로 교체 (필수)
- 예제의 다양성 유지: 모든 테두리 스타일, 다양한 arrow 형태, CJK 텍스트, overflow 데모 포함

## 코드 변경 시 문서 업데이트

- DSL 문법, 옵션, 커맨드 변경 시 `printGuide()` 업데이트 검토 (필수)
- CLI 인터페이스, 서브커맨드 변경 시 help 메시지 업데이트 검토 (필수)
