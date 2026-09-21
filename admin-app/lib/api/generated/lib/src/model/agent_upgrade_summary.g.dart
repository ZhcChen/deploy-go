// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_upgrade_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentUpgradeSummary extends AgentUpgradeSummary {
  @override
  final String? currentVersion;
  @override
  final String? errorCode;
  @override
  final String? errorSummary;
  @override
  final String? jobId;
  @override
  final String? phase;
  @override
  final String state;
  @override
  final String? targetVersion;
  @override
  final String? updatedAt;

  factory _$AgentUpgradeSummary([
    void Function(AgentUpgradeSummaryBuilder)? updates,
  ]) => (AgentUpgradeSummaryBuilder()..update(updates))._build();

  _$AgentUpgradeSummary._({
    this.currentVersion,
    this.errorCode,
    this.errorSummary,
    this.jobId,
    this.phase,
    required this.state,
    this.targetVersion,
    this.updatedAt,
  }) : super._();
  @override
  AgentUpgradeSummary rebuild(
    void Function(AgentUpgradeSummaryBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  AgentUpgradeSummaryBuilder toBuilder() =>
      AgentUpgradeSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentUpgradeSummary &&
        currentVersion == other.currentVersion &&
        errorCode == other.errorCode &&
        errorSummary == other.errorSummary &&
        jobId == other.jobId &&
        phase == other.phase &&
        state == other.state &&
        targetVersion == other.targetVersion &&
        updatedAt == other.updatedAt;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, currentVersion.hashCode);
    _$hash = $jc(_$hash, errorCode.hashCode);
    _$hash = $jc(_$hash, errorSummary.hashCode);
    _$hash = $jc(_$hash, jobId.hashCode);
    _$hash = $jc(_$hash, phase.hashCode);
    _$hash = $jc(_$hash, state.hashCode);
    _$hash = $jc(_$hash, targetVersion.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentUpgradeSummary')
          ..add('currentVersion', currentVersion)
          ..add('errorCode', errorCode)
          ..add('errorSummary', errorSummary)
          ..add('jobId', jobId)
          ..add('phase', phase)
          ..add('state', state)
          ..add('targetVersion', targetVersion)
          ..add('updatedAt', updatedAt))
        .toString();
  }
}

class AgentUpgradeSummaryBuilder
    implements Builder<AgentUpgradeSummary, AgentUpgradeSummaryBuilder> {
  _$AgentUpgradeSummary? _$v;

  String? _currentVersion;
  String? get currentVersion => _$this._currentVersion;
  set currentVersion(String? currentVersion) =>
      _$this._currentVersion = currentVersion;

  String? _errorCode;
  String? get errorCode => _$this._errorCode;
  set errorCode(String? errorCode) => _$this._errorCode = errorCode;

  String? _errorSummary;
  String? get errorSummary => _$this._errorSummary;
  set errorSummary(String? errorSummary) => _$this._errorSummary = errorSummary;

  String? _jobId;
  String? get jobId => _$this._jobId;
  set jobId(String? jobId) => _$this._jobId = jobId;

  String? _phase;
  String? get phase => _$this._phase;
  set phase(String? phase) => _$this._phase = phase;

  String? _state;
  String? get state => _$this._state;
  set state(String? state) => _$this._state = state;

  String? _targetVersion;
  String? get targetVersion => _$this._targetVersion;
  set targetVersion(String? targetVersion) =>
      _$this._targetVersion = targetVersion;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  AgentUpgradeSummaryBuilder() {
    AgentUpgradeSummary._defaults(this);
  }

  AgentUpgradeSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _currentVersion = $v.currentVersion;
      _errorCode = $v.errorCode;
      _errorSummary = $v.errorSummary;
      _jobId = $v.jobId;
      _phase = $v.phase;
      _state = $v.state;
      _targetVersion = $v.targetVersion;
      _updatedAt = $v.updatedAt;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentUpgradeSummary other) {
    _$v = other as _$AgentUpgradeSummary;
  }

  @override
  void update(void Function(AgentUpgradeSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentUpgradeSummary build() => _build();

  _$AgentUpgradeSummary _build() {
    final _$result =
        _$v ??
        _$AgentUpgradeSummary._(
          currentVersion: currentVersion,
          errorCode: errorCode,
          errorSummary: errorSummary,
          jobId: jobId,
          phase: phase,
          state: BuiltValueNullFieldError.checkNotNull(
            state,
            r'AgentUpgradeSummary',
            'state',
          ),
          targetVersion: targetVersion,
          updatedAt: updatedAt,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
