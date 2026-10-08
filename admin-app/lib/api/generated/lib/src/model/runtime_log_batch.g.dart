// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'runtime_log_batch.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RuntimeLogBatch extends RuntimeLogBatch {
  @override
  final int after;
  @override
  final String component;
  @override
  final BuiltList<RuntimeLogEntrySchema> entries;
  @override
  final String epoch;
  @override
  final int evictedBytes;
  @override
  final int minimum;

  factory _$RuntimeLogBatch([void Function(RuntimeLogBatchBuilder)? updates]) =>
      (RuntimeLogBatchBuilder()..update(updates))._build();

  _$RuntimeLogBatch._({
    required this.after,
    required this.component,
    required this.entries,
    required this.epoch,
    required this.evictedBytes,
    required this.minimum,
  }) : super._();
  @override
  RuntimeLogBatch rebuild(void Function(RuntimeLogBatchBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RuntimeLogBatchBuilder toBuilder() => RuntimeLogBatchBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RuntimeLogBatch &&
        after == other.after &&
        component == other.component &&
        entries == other.entries &&
        epoch == other.epoch &&
        evictedBytes == other.evictedBytes &&
        minimum == other.minimum;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, after.hashCode);
    _$hash = $jc(_$hash, component.hashCode);
    _$hash = $jc(_$hash, entries.hashCode);
    _$hash = $jc(_$hash, epoch.hashCode);
    _$hash = $jc(_$hash, evictedBytes.hashCode);
    _$hash = $jc(_$hash, minimum.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RuntimeLogBatch')
          ..add('after', after)
          ..add('component', component)
          ..add('entries', entries)
          ..add('epoch', epoch)
          ..add('evictedBytes', evictedBytes)
          ..add('minimum', minimum))
        .toString();
  }
}

class RuntimeLogBatchBuilder
    implements Builder<RuntimeLogBatch, RuntimeLogBatchBuilder> {
  _$RuntimeLogBatch? _$v;

  int? _after;
  int? get after => _$this._after;
  set after(int? after) => _$this._after = after;

  String? _component;
  String? get component => _$this._component;
  set component(String? component) => _$this._component = component;

  ListBuilder<RuntimeLogEntrySchema>? _entries;
  ListBuilder<RuntimeLogEntrySchema> get entries =>
      _$this._entries ??= ListBuilder<RuntimeLogEntrySchema>();
  set entries(ListBuilder<RuntimeLogEntrySchema>? entries) =>
      _$this._entries = entries;

  String? _epoch;
  String? get epoch => _$this._epoch;
  set epoch(String? epoch) => _$this._epoch = epoch;

  int? _evictedBytes;
  int? get evictedBytes => _$this._evictedBytes;
  set evictedBytes(int? evictedBytes) => _$this._evictedBytes = evictedBytes;

  int? _minimum;
  int? get minimum => _$this._minimum;
  set minimum(int? minimum) => _$this._minimum = minimum;

  RuntimeLogBatchBuilder() {
    RuntimeLogBatch._defaults(this);
  }

  RuntimeLogBatchBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _after = $v.after;
      _component = $v.component;
      _entries = $v.entries.toBuilder();
      _epoch = $v.epoch;
      _evictedBytes = $v.evictedBytes;
      _minimum = $v.minimum;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RuntimeLogBatch other) {
    _$v = other as _$RuntimeLogBatch;
  }

  @override
  void update(void Function(RuntimeLogBatchBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RuntimeLogBatch build() => _build();

  _$RuntimeLogBatch _build() {
    _$RuntimeLogBatch _$result;
    try {
      _$result =
          _$v ??
          _$RuntimeLogBatch._(
            after: BuiltValueNullFieldError.checkNotNull(
              after,
              r'RuntimeLogBatch',
              'after',
            ),
            component: BuiltValueNullFieldError.checkNotNull(
              component,
              r'RuntimeLogBatch',
              'component',
            ),
            entries: entries.build(),
            epoch: BuiltValueNullFieldError.checkNotNull(
              epoch,
              r'RuntimeLogBatch',
              'epoch',
            ),
            evictedBytes: BuiltValueNullFieldError.checkNotNull(
              evictedBytes,
              r'RuntimeLogBatch',
              'evictedBytes',
            ),
            minimum: BuiltValueNullFieldError.checkNotNull(
              minimum,
              r'RuntimeLogBatch',
              'minimum',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'entries';
        entries.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'RuntimeLogBatch',
          _$failedField,
          e.toString(),
        );
      }
      rethrow;
    }
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
