// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'runtime_log_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RuntimeLogResponse extends RuntimeLogResponse {
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

  factory _$RuntimeLogResponse([
    void Function(RuntimeLogResponseBuilder)? updates,
  ]) => (RuntimeLogResponseBuilder()..update(updates))._build();

  _$RuntimeLogResponse._({
    required this.fields,
    required this.level,
    required this.message,
    this.requestId,
    required this.sequence,
    required this.target,
    required this.timestamp,
  }) : super._();
  @override
  RuntimeLogResponse rebuild(
    void Function(RuntimeLogResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  RuntimeLogResponseBuilder toBuilder() =>
      RuntimeLogResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RuntimeLogResponse &&
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
    return (newBuiltValueToStringHelper(r'RuntimeLogResponse')
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

class RuntimeLogResponseBuilder
    implements Builder<RuntimeLogResponse, RuntimeLogResponseBuilder> {
  _$RuntimeLogResponse? _$v;

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

  RuntimeLogResponseBuilder() {
    RuntimeLogResponse._defaults(this);
  }

  RuntimeLogResponseBuilder get _$this {
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
  void replace(RuntimeLogResponse other) {
    _$v = other as _$RuntimeLogResponse;
  }

  @override
  void update(void Function(RuntimeLogResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RuntimeLogResponse build() => _build();

  _$RuntimeLogResponse _build() {
    _$RuntimeLogResponse _$result;
    try {
      _$result =
          _$v ??
          _$RuntimeLogResponse._(
            fields: fields.build(),
            level: BuiltValueNullFieldError.checkNotNull(
              level,
              r'RuntimeLogResponse',
              'level',
            ),
            message: BuiltValueNullFieldError.checkNotNull(
              message,
              r'RuntimeLogResponse',
              'message',
            ),
            requestId: requestId,
            sequence: BuiltValueNullFieldError.checkNotNull(
              sequence,
              r'RuntimeLogResponse',
              'sequence',
            ),
            target: BuiltValueNullFieldError.checkNotNull(
              target,
              r'RuntimeLogResponse',
              'target',
            ),
            timestamp: BuiltValueNullFieldError.checkNotNull(
              timestamp,
              r'RuntimeLogResponse',
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
          r'RuntimeLogResponse',
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
