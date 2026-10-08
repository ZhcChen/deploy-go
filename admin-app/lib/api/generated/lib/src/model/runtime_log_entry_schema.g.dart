// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'runtime_log_entry_schema.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RuntimeLogEntrySchema extends RuntimeLogEntrySchema {
  @override
  final BuiltMap<String, JsonObject?> fields;
  @override
  final String level;
  @override
  final String message;
  @override
  final String? requestId;
  @override
  final int sequence;
  @override
  final String target;
  @override
  final String timestamp;

  factory _$RuntimeLogEntrySchema([
    void Function(RuntimeLogEntrySchemaBuilder)? updates,
  ]) => (RuntimeLogEntrySchemaBuilder()..update(updates))._build();

  _$RuntimeLogEntrySchema._({
    required this.fields,
    required this.level,
    required this.message,
    this.requestId,
    required this.sequence,
    required this.target,
    required this.timestamp,
  }) : super._();
  @override
  RuntimeLogEntrySchema rebuild(
    void Function(RuntimeLogEntrySchemaBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RuntimeLogEntrySchemaBuilder toBuilder() =>
      RuntimeLogEntrySchemaBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RuntimeLogEntrySchema &&
        fields == other.fields &&
        level == other.level &&
        message == other.message &&
        requestId == other.requestId &&
        sequence == other.sequence &&
        target == other.target &&
        timestamp == other.timestamp;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, fields.hashCode);
    _$hash = $jc(_$hash, level.hashCode);
    _$hash = $jc(_$hash, message.hashCode);
    _$hash = $jc(_$hash, requestId.hashCode);
    _$hash = $jc(_$hash, sequence.hashCode);
    _$hash = $jc(_$hash, target.hashCode);
    _$hash = $jc(_$hash, timestamp.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'RuntimeLogEntrySchema')
          ..add('fields', fields)
          ..add('level', level)
          ..add('message', message)
          ..add('requestId', requestId)
          ..add('sequence', sequence)
          ..add('target', target)
          ..add('timestamp', timestamp))
        .toString();
  }
}

class RuntimeLogEntrySchemaBuilder
    implements Builder<RuntimeLogEntrySchema, RuntimeLogEntrySchemaBuilder> {
  _$RuntimeLogEntrySchema? _$v;

  MapBuilder<String, JsonObject?>? _fields;
  MapBuilder<String, JsonObject?> get fields =>
      _$this._fields ??= MapBuilder<String, JsonObject?>();
  set fields(MapBuilder<String, JsonObject?>? fields) =>
      _$this._fields = fields;

  String? _level;
  String? get level => _$this._level;
  set level(String? level) => _$this._level = level;

  String? _message;
  String? get message => _$this._message;
  set message(String? message) => _$this._message = message;

  String? _requestId;
  String? get requestId => _$this._requestId;
  set requestId(String? requestId) => _$this._requestId = requestId;

  int? _sequence;
  int? get sequence => _$this._sequence;
  set sequence(int? sequence) => _$this._sequence = sequence;

  String? _target;
  String? get target => _$this._target;
  set target(String? target) => _$this._target = target;

  String? _timestamp;
  String? get timestamp => _$this._timestamp;
  set timestamp(String? timestamp) => _$this._timestamp = timestamp;

  RuntimeLogEntrySchemaBuilder() {
    RuntimeLogEntrySchema._defaults(this);
  }

  RuntimeLogEntrySchemaBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _fields = $v.fields.toBuilder();
      _level = $v.level;
      _message = $v.message;
      _requestId = $v.requestId;
      _sequence = $v.sequence;
      _target = $v.target;
      _timestamp = $v.timestamp;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RuntimeLogEntrySchema other) {
    _$v = other as _$RuntimeLogEntrySchema;
  }

  @override
  void update(void Function(RuntimeLogEntrySchemaBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RuntimeLogEntrySchema build() => _build();

  _$RuntimeLogEntrySchema _build() {
    _$RuntimeLogEntrySchema _$result;
    try {
      _$result =
          _$v ??
          _$RuntimeLogEntrySchema._(
            fields: fields.build(),
            level: BuiltValueNullFieldError.checkNotNull(
              level,
              r'RuntimeLogEntrySchema',
              'level',
            ),
            message: BuiltValueNullFieldError.checkNotNull(
              message,
              r'RuntimeLogEntrySchema',
              'message',
            ),
            requestId: requestId,
            sequence: BuiltValueNullFieldError.checkNotNull(
              sequence,
              r'RuntimeLogEntrySchema',
              'sequence',
            ),
            target: BuiltValueNullFieldError.checkNotNull(
              target,
              r'RuntimeLogEntrySchema',
              'target',
            ),
            timestamp: BuiltValueNullFieldError.checkNotNull(
              timestamp,
              r'RuntimeLogEntrySchema',
              'timestamp',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'fields';
        fields.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'RuntimeLogEntrySchema',
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
