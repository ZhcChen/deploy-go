// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_log_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentLogResponse extends DeploymentLogResponse {
  @override
  final String content;
  @override
  final String createdAt;
  @override
  final int sequence;
  @override
  final String? stage;
  @override
  final String stream;
  @override
  final String? taskId;
  @override
  final int? taskSequence;
  @override
  final bool truncated;

  factory _$DeploymentLogResponse([
    void Function(DeploymentLogResponseBuilder)? updates,
  ]) => (DeploymentLogResponseBuilder()..update(updates))._build();

  _$DeploymentLogResponse._({
    required this.content,
    required this.createdAt,
    required this.sequence,
    this.stage,
    required this.stream,
    this.taskId,
    this.taskSequence,
    required this.truncated,
  }) : super._();
  @override
  DeploymentLogResponse rebuild(
    void Function(DeploymentLogResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeploymentLogResponseBuilder toBuilder() =>
      DeploymentLogResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentLogResponse &&
        content == other.content &&
        createdAt == other.createdAt &&
        sequence == other.sequence &&
        stage == other.stage &&
        stream == other.stream &&
        taskId == other.taskId &&
        taskSequence == other.taskSequence &&
        truncated == other.truncated;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, content.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, sequence.hashCode);
    _$hash = $jc(_$hash, stage.hashCode);
    _$hash = $jc(_$hash, stream.hashCode);
    _$hash = $jc(_$hash, taskId.hashCode);
    _$hash = $jc(_$hash, taskSequence.hashCode);
    _$hash = $jc(_$hash, truncated.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeploymentLogResponse')
          ..add('content', content)
          ..add('createdAt', createdAt)
          ..add('sequence', sequence)
          ..add('stage', stage)
          ..add('stream', stream)
          ..add('taskId', taskId)
          ..add('taskSequence', taskSequence)
          ..add('truncated', truncated))
        .toString();
  }
}

class DeploymentLogResponseBuilder
    implements Builder<DeploymentLogResponse, DeploymentLogResponseBuilder> {
  _$DeploymentLogResponse? _$v;

  String? _content;
  String? get content => _$this._content;
  set content(String? content) => _$this._content = content;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  int? _sequence;
  int? get sequence => _$this._sequence;
  set sequence(int? sequence) => _$this._sequence = sequence;

  String? _stage;
  String? get stage => _$this._stage;
  set stage(String? stage) => _$this._stage = stage;

  String? _stream;
  String? get stream => _$this._stream;
  set stream(String? stream) => _$this._stream = stream;

  String? _taskId;
  String? get taskId => _$this._taskId;
  set taskId(String? taskId) => _$this._taskId = taskId;

  int? _taskSequence;
  int? get taskSequence => _$this._taskSequence;
  set taskSequence(int? taskSequence) => _$this._taskSequence = taskSequence;

  bool? _truncated;
  bool? get truncated => _$this._truncated;
  set truncated(bool? truncated) => _$this._truncated = truncated;

  DeploymentLogResponseBuilder() {
    DeploymentLogResponse._defaults(this);
  }

  DeploymentLogResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _content = $v.content;
      _createdAt = $v.createdAt;
      _sequence = $v.sequence;
      _stage = $v.stage;
      _stream = $v.stream;
      _taskId = $v.taskId;
      _taskSequence = $v.taskSequence;
      _truncated = $v.truncated;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentLogResponse other) {
    _$v = other as _$DeploymentLogResponse;
  }

  @override
  void update(void Function(DeploymentLogResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentLogResponse build() => _build();

  _$DeploymentLogResponse _build() {
    final _$result =
        _$v ??
        _$DeploymentLogResponse._(
          content: BuiltValueNullFieldError.checkNotNull(
            content,
            r'DeploymentLogResponse',
            'content',
          ),
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'DeploymentLogResponse',
            'createdAt',
          ),
          sequence: BuiltValueNullFieldError.checkNotNull(
            sequence,
            r'DeploymentLogResponse',
            'sequence',
          ),
          stage: stage,
          stream: BuiltValueNullFieldError.checkNotNull(
            stream,
            r'DeploymentLogResponse',
            'stream',
          ),
          taskId: taskId,
          taskSequence: taskSequence,
          truncated: BuiltValueNullFieldError.checkNotNull(
            truncated,
            r'DeploymentLogResponse',
            'truncated',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
