// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'deployment_stage_task_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$DeploymentStageTaskSummary extends DeploymentStageTaskSummary {
  @override
  final String createdAt;
  @override
  final String? errorCode;
  @override
  final int? exitCode;
  @override
  final String? finishedAt;
  @override
  final String stage;
  @override
  final String? startedAt;
  @override
  final String status;
  @override
  final String taskId;
  @override
  final String updatedAt;

  factory _$DeploymentStageTaskSummary([
    void Function(DeploymentStageTaskSummaryBuilder)? updates,
  ]) => (DeploymentStageTaskSummaryBuilder()..update(updates))._build();

  _$DeploymentStageTaskSummary._({
    required this.createdAt,
    this.errorCode,
    this.exitCode,
    this.finishedAt,
    required this.stage,
    this.startedAt,
    required this.status,
    required this.taskId,
    required this.updatedAt,
  }) : super._();
  @override
  DeploymentStageTaskSummary rebuild(
    void Function(DeploymentStageTaskSummaryBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  DeploymentStageTaskSummaryBuilder toBuilder() =>
      DeploymentStageTaskSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is DeploymentStageTaskSummary &&
        createdAt == other.createdAt &&
        errorCode == other.errorCode &&
        exitCode == other.exitCode &&
        finishedAt == other.finishedAt &&
        stage == other.stage &&
        startedAt == other.startedAt &&
        status == other.status &&
        taskId == other.taskId &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, errorCode.hashCode);
    _$hash = $jc(_$hash, exitCode.hashCode);
    _$hash = $jc(_$hash, finishedAt.hashCode);
    _$hash = $jc(_$hash, stage.hashCode);
    _$hash = $jc(_$hash, startedAt.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, taskId.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'DeploymentStageTaskSummary')
          ..add('createdAt', createdAt)
          ..add('errorCode', errorCode)
          ..add('exitCode', exitCode)
          ..add('finishedAt', finishedAt)
          ..add('stage', stage)
          ..add('startedAt', startedAt)
          ..add('status', status)
          ..add('taskId', taskId)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class DeploymentStageTaskSummaryBuilder
    implements
        Builder<DeploymentStageTaskSummary, DeploymentStageTaskSummaryBuilder> {
  _$DeploymentStageTaskSummary? _$v;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _errorCode;
  String? get errorCode => _$this._errorCode;
  set errorCode(String? errorCode) => _$this._errorCode = errorCode;

  int? _exitCode;
  int? get exitCode => _$this._exitCode;
  set exitCode(int? exitCode) => _$this._exitCode = exitCode;

  String? _finishedAt;
  String? get finishedAt => _$this._finishedAt;
  set finishedAt(String? finishedAt) => _$this._finishedAt = finishedAt;

  String? _stage;
  String? get stage => _$this._stage;
  set stage(String? stage) => _$this._stage = stage;

  String? _startedAt;
  String? get startedAt => _$this._startedAt;
  set startedAt(String? startedAt) => _$this._startedAt = startedAt;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _taskId;
  String? get taskId => _$this._taskId;
  set taskId(String? taskId) => _$this._taskId = taskId;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  DeploymentStageTaskSummaryBuilder() {
    DeploymentStageTaskSummary._defaults(this);
  }

  DeploymentStageTaskSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _errorCode = $v.errorCode;
      _exitCode = $v.exitCode;
      _finishedAt = $v.finishedAt;
      _stage = $v.stage;
      _startedAt = $v.startedAt;
      _status = $v.status;
      _taskId = $v.taskId;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(DeploymentStageTaskSummary other) {
    _$v = other as _$DeploymentStageTaskSummary;
  }

  @override
  void update(void Function(DeploymentStageTaskSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  DeploymentStageTaskSummary build() => _build();

  _$DeploymentStageTaskSummary _build() {
    final _$result =
        _$v ??
        _$DeploymentStageTaskSummary._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'DeploymentStageTaskSummary',
            'createdAt',
          ),
          errorCode: errorCode,
          exitCode: exitCode,
          finishedAt: finishedAt,
          stage: BuiltValueNullFieldError.checkNotNull(
            stage,
            r'DeploymentStageTaskSummary',
            'stage',
          ),
          startedAt: startedAt,
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'DeploymentStageTaskSummary',
            'status',
          ),
          taskId: BuiltValueNullFieldError.checkNotNull(
            taskId,
            r'DeploymentStageTaskSummary',
            'taskId',
          ),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'DeploymentStageTaskSummary',
            'updatedAt',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
