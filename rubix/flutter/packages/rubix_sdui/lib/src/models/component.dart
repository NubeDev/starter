/// Sealed `SduiComponent` IR — discriminated on the `type` JSON field.
///
/// Mirrors the `Component` enum in
/// `crates/starter-ui-ir/src/component.rs` (39 portable variants on
/// IR v5). Unknown / malformed `type` values degrade to
/// [DanglingComponent] rather than throwing.
///
/// **Scaffold only.** Variant bodies are stubs — they store the raw
/// JSON map so the dispatcher can identify them. Field-level
/// parsing lands in stage F3 (see PENDING.md), preferably from a
/// script that consumes
/// `crates/starter-ui-ir/schema/starter-ui-ir.schema.json` so we
/// don't hand-transliterate 39 variants.
///
/// Pure Dart — no Flutter imports.
library;

sealed class SduiComponent {
  const SduiComponent({required this.id, required this.raw});

  /// Stable id assigned by the resolver. May be empty for root.
  final String id;

  /// Raw JSON map — every variant holds this for round-trip + while
  /// field-level parsers are unimplemented. Remove the field once
  /// individual variants own their typed fields.
  final Map<String, Object?> raw;

  /// Wire discriminator (e.g. `"page"`, `"row"`, `"button"`).
  String get type;

  Map<String, Object?> toJson() => {...raw, 'type': type, 'id': id};

  /// Parses any component variant. Unknown `type` → [DanglingComponent].
  factory SduiComponent.fromJson(Map<String, Object?> map) {
    final type = map['type'] as String? ?? '';
    final id = map['id'] as String? ?? '';
    return switch (type) {
      // ---- layout ----
      'page' => PageComponent(id: id, raw: map),
      'row' => RowComponent(id: id, raw: map),
      'col' => ColComponent(id: id, raw: map),
      'grid' => GridComponent(id: id, raw: map),
      'tabs' => TabsComponent(id: id, raw: map),
      'repeat' => RepeatComponent(id: id, raw: map),
      'section' => SectionComponent(id: id, raw: map),
      'divider' => DividerComponent(id: id, raw: map),
      'field_group' => FieldGroupComponent(id: id, raw: map),
      // ---- display ----
      'text' => TextComponent(id: id, raw: map),
      'heading' => HeadingComponent(id: id, raw: map),
      'badge' => BadgeComponent(id: id, raw: map),
      'diff' => DiffComponent(id: id, raw: map),
      'markdown' => MarkdownComponent(id: id, raw: map),
      'kpi' => KpiComponent(id: id, raw: map),
      'kpi_grid' => KpiGridComponent(id: id, raw: map),
      'chart' => ChartComponent(id: id, raw: map),
      'sparkline' => SparklineComponent(id: id, raw: map),
      // ---- data ----
      'table' => TableComponent(id: id, raw: map),
      'array_table' => ArrayTableComponent(id: id, raw: map),
      'json_table' => JsonTableComponent(id: id, raw: map),
      'list' => ListComponent(id: id, raw: map),
      'detail' => DetailComponent(id: id, raw: map),
      'tree' => TreeComponent(id: id, raw: map),
      'timeline' => TimelineComponent(id: id, raw: map),
      // ---- input ----
      'toggle' => ToggleComponent(id: id, raw: map),
      'slider' => SliderComponent(id: id, raw: map),
      'select' => SelectComponent(id: id, raw: map),
      'text_field' => TextFieldComponent(id: id, raw: map),
      'number_field' => NumberFieldComponent(id: id, raw: map),
      'textarea' => TextareaComponent(id: id, raw: map),
      'select_field' => SelectFieldComponent(id: id, raw: map),
      'radio_group' => RadioGroupComponent(id: id, raw: map),
      'segmented' => SegmentedComponent(id: id, raw: map),
      'date_field' => DateFieldComponent(id: id, raw: map),
      'date_range' => DateRangeComponent(id: id, raw: map),
      'checkbox' => CheckboxComponent(id: id, raw: map),
      'ref_picker' => RefPickerComponent(id: id, raw: map),
      'rich_text' => RichTextComponent(id: id, raw: map),
      'markdown_editor' => MarkdownEditorComponent(id: id, raw: map),
      // ---- interactive ----
      'button' => ButtonComponent(id: id, raw: map),
      'drawer' => DrawerComponent(id: id, raw: map),
      'dialog' => DialogComponent(id: id, raw: map),
      'menu' => MenuComponent(id: id, raw: map),
      // ---- composite ----
      'form' => FormComponent(id: id, raw: map),
      'card' => CardComponent(id: id, raw: map),
      'wizard' => WizardComponent(id: id, raw: map),
      // ---- sentinels / escapes ----
      'custom' => CustomComponent(id: id, raw: map),
      'action_widget' => ActionWidgetComponent(id: id, raw: map),
      'forbidden' => ForbiddenComponent(id: id, raw: map),
      'dangling' => DanglingComponent(id: id, raw: map, reason: 'server'),
      // Unknown variants degrade — never throw.
      _ => DanglingComponent(id: id, raw: map, reason: 'unknown:$type'),
    };
  }
}

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

final class PageComponent extends SduiComponent {
  const PageComponent({required super.id, required super.raw});
  @override
  String get type => 'page';
}

final class RowComponent extends SduiComponent {
  const RowComponent({required super.id, required super.raw});
  @override
  String get type => 'row';
}

final class ColComponent extends SduiComponent {
  const ColComponent({required super.id, required super.raw});
  @override
  String get type => 'col';
}

final class GridComponent extends SduiComponent {
  const GridComponent({required super.id, required super.raw});
  @override
  String get type => 'grid';
}

final class TabsComponent extends SduiComponent {
  const TabsComponent({required super.id, required super.raw});
  @override
  String get type => 'tabs';
}

final class RepeatComponent extends SduiComponent {
  const RepeatComponent({required super.id, required super.raw});
  @override
  String get type => 'repeat';
}

final class SectionComponent extends SduiComponent {
  const SectionComponent({required super.id, required super.raw});
  @override
  String get type => 'section';
}

final class DividerComponent extends SduiComponent {
  const DividerComponent({required super.id, required super.raw});
  @override
  String get type => 'divider';
}

final class FieldGroupComponent extends SduiComponent {
  const FieldGroupComponent({required super.id, required super.raw});
  @override
  String get type => 'field_group';
}

// ---------------------------------------------------------------------------
// Display
// ---------------------------------------------------------------------------

final class TextComponent extends SduiComponent {
  const TextComponent({required super.id, required super.raw});
  @override
  String get type => 'text';
}

final class HeadingComponent extends SduiComponent {
  const HeadingComponent({required super.id, required super.raw});
  @override
  String get type => 'heading';
}

final class BadgeComponent extends SduiComponent {
  const BadgeComponent({required super.id, required super.raw});
  @override
  String get type => 'badge';
}

final class DiffComponent extends SduiComponent {
  const DiffComponent({required super.id, required super.raw});
  @override
  String get type => 'diff';
}

final class MarkdownComponent extends SduiComponent {
  const MarkdownComponent({required super.id, required super.raw});
  @override
  String get type => 'markdown';
}

final class KpiComponent extends SduiComponent {
  const KpiComponent({required super.id, required super.raw});
  @override
  String get type => 'kpi';
}

final class KpiGridComponent extends SduiComponent {
  const KpiGridComponent({required super.id, required super.raw});
  @override
  String get type => 'kpi_grid';
}

final class ChartComponent extends SduiComponent {
  const ChartComponent({required super.id, required super.raw});
  @override
  String get type => 'chart';
}

final class SparklineComponent extends SduiComponent {
  const SparklineComponent({required super.id, required super.raw});
  @override
  String get type => 'sparkline';
}

// ---------------------------------------------------------------------------
// Data
// ---------------------------------------------------------------------------

final class TableComponent extends SduiComponent {
  const TableComponent({required super.id, required super.raw});
  @override
  String get type => 'table';
}

final class ArrayTableComponent extends SduiComponent {
  const ArrayTableComponent({required super.id, required super.raw});
  @override
  String get type => 'array_table';
}

final class JsonTableComponent extends SduiComponent {
  const JsonTableComponent({required super.id, required super.raw});
  @override
  String get type => 'json_table';
}

final class ListComponent extends SduiComponent {
  const ListComponent({required super.id, required super.raw});
  @override
  String get type => 'list';
}

final class DetailComponent extends SduiComponent {
  const DetailComponent({required super.id, required super.raw});
  @override
  String get type => 'detail';
}

final class TreeComponent extends SduiComponent {
  const TreeComponent({required super.id, required super.raw});
  @override
  String get type => 'tree';
}

final class TimelineComponent extends SduiComponent {
  const TimelineComponent({required super.id, required super.raw});
  @override
  String get type => 'timeline';
}

// ---------------------------------------------------------------------------
// Input
// ---------------------------------------------------------------------------

final class ToggleComponent extends SduiComponent {
  const ToggleComponent({required super.id, required super.raw});
  @override
  String get type => 'toggle';
}

final class SliderComponent extends SduiComponent {
  const SliderComponent({required super.id, required super.raw});
  @override
  String get type => 'slider';
}

final class SelectComponent extends SduiComponent {
  const SelectComponent({required super.id, required super.raw});
  @override
  String get type => 'select';
}

final class TextFieldComponent extends SduiComponent {
  const TextFieldComponent({required super.id, required super.raw});
  @override
  String get type => 'text_field';
}

final class NumberFieldComponent extends SduiComponent {
  const NumberFieldComponent({required super.id, required super.raw});
  @override
  String get type => 'number_field';
}

final class TextareaComponent extends SduiComponent {
  const TextareaComponent({required super.id, required super.raw});
  @override
  String get type => 'textarea';
}

final class SelectFieldComponent extends SduiComponent {
  const SelectFieldComponent({required super.id, required super.raw});
  @override
  String get type => 'select_field';
}

final class RadioGroupComponent extends SduiComponent {
  const RadioGroupComponent({required super.id, required super.raw});
  @override
  String get type => 'radio_group';
}

final class SegmentedComponent extends SduiComponent {
  const SegmentedComponent({required super.id, required super.raw});
  @override
  String get type => 'segmented';
}

final class DateFieldComponent extends SduiComponent {
  const DateFieldComponent({required super.id, required super.raw});
  @override
  String get type => 'date_field';
}

final class DateRangeComponent extends SduiComponent {
  const DateRangeComponent({required super.id, required super.raw});
  @override
  String get type => 'date_range';
}

final class CheckboxComponent extends SduiComponent {
  const CheckboxComponent({required super.id, required super.raw});
  @override
  String get type => 'checkbox';
}

final class RefPickerComponent extends SduiComponent {
  const RefPickerComponent({required super.id, required super.raw});
  @override
  String get type => 'ref_picker';
}

final class RichTextComponent extends SduiComponent {
  const RichTextComponent({required super.id, required super.raw});
  @override
  String get type => 'rich_text';
}

final class MarkdownEditorComponent extends SduiComponent {
  const MarkdownEditorComponent({required super.id, required super.raw});
  @override
  String get type => 'markdown_editor';
}

// ---------------------------------------------------------------------------
// Interactive
// ---------------------------------------------------------------------------

final class ButtonComponent extends SduiComponent {
  const ButtonComponent({required super.id, required super.raw});
  @override
  String get type => 'button';
}

final class DrawerComponent extends SduiComponent {
  const DrawerComponent({required super.id, required super.raw});
  @override
  String get type => 'drawer';
}

final class DialogComponent extends SduiComponent {
  const DialogComponent({required super.id, required super.raw});
  @override
  String get type => 'dialog';
}

final class MenuComponent extends SduiComponent {
  const MenuComponent({required super.id, required super.raw});
  @override
  String get type => 'menu';
}

// ---------------------------------------------------------------------------
// Composite
// ---------------------------------------------------------------------------

final class FormComponent extends SduiComponent {
  const FormComponent({required super.id, required super.raw});
  @override
  String get type => 'form';
}

final class CardComponent extends SduiComponent {
  const CardComponent({required super.id, required super.raw});
  @override
  String get type => 'card';
}

final class WizardComponent extends SduiComponent {
  const WizardComponent({required super.id, required super.raw});
  @override
  String get type => 'wizard';
}

// ---------------------------------------------------------------------------
// Sentinels / escape hatches
// ---------------------------------------------------------------------------

final class CustomComponent extends SduiComponent {
  const CustomComponent({required super.id, required super.raw});
  @override
  String get type => 'custom';

  /// Renderer key looked up in `CustomRendererRegistry`.
  String? get rendererId => raw['renderer_id'] as String?;

  /// Opaque props forwarded to the renderer.
  Object? get props => raw['props'];
}

final class ActionWidgetComponent extends SduiComponent {
  const ActionWidgetComponent({required super.id, required super.raw});
  @override
  String get type => 'action_widget';
}

final class ForbiddenComponent extends SduiComponent {
  const ForbiddenComponent({required super.id, required super.raw});
  @override
  String get type => 'forbidden';
}

final class DanglingComponent extends SduiComponent {
  const DanglingComponent({
    required super.id,
    required super.raw,
    required this.reason,
  });
  @override
  String get type => 'dangling';

  /// Tag indicating why the component degraded — e.g. `"server"`,
  /// `"unknown:foobar"`, `"binding_miss"`.
  final String reason;
}
