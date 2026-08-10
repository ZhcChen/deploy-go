// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'terminal_session_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TerminalSessionResponse extends TerminalSessionResponse {
  @override
  final String actorId;
  @override
  final String agentId;
  @override
  final String? closeRequestedAt;
  @override
  final int? exitCode;
  @override
  final String? exitReason;
  @override
  final String? finishedAt;
  @override
  final String id;
  @override
  final int inputBytes;
  @override
  final String nodeId;
  @override
  final String? openedAt;
  @override
  final int outputBytes;
  @override
  final String startedAt;
  @override
  final String status;

  factory _$TerminalSessionResponse([
    void Function(TerminalSessionResponseBuilder)? updates,
  ]) => (TerminalSessionResponseBuilder()..update(updates))._build();

  _$TerminalSessionResponse._({
    required this.actorId,
    required this.agentId,
    this.closeRequestedAt,
    this.exitCode,
    this.exitReason,
    this.finishedAt,
    required this.id,
    required this.inputBytes,
    required this.nodeId,
    this.openedAt,
    required this.outputBytes,
    required this.startedAt,
    required this.status,
  }) : super._();
  @override
  TerminalSessionResponse rebuild(
    void Function(TerminalSessionResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  TerminalSessionResponseBuilder toBuilder() =>
      TerminalSessionResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TerminalSessionResponse &&
        actorId == other.actorId &&
        agentId == other.agentId &&
        closeRequestedAt == other.closeRequestedAt &&
        exitCode == other.exitCode &&
        exitReason == other.exitReason &&
        finishedAt == other.finishedAt &&
        id == other.id &&
        inputBytes == other.inputBytes &&
        nodeId == other.nodeId &&
        openedAt == other.openedAt &&
        outputBytes == other.outputBytes &&
        startedAt == other.startedAt &&
        status == other.status;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, actorId.hashCode);
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, closeRequestedAt.hashCode);
    _$hash = $jc(_$hash, exitCode.hashCode);
    _$hash = $jc(_$hash, exitReason.hashCode);
    _$hash = $jc(_$hash, finishedAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, inputBytes.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, openedAt.hashCode);
    _$hash = $jc(_$hash, outputBytes.hashCode);
    _$hash = $jc(_$hash, startedAt.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TerminalSessionResponse')
          ..add('actorId', actorId)
          ..add('agentId', agentId)
          ..add('closeRequestedAt', closeRequestedAt)
          ..add('exitCode', exitCode)
          ..add('exitReason', exitReason)
          ..add('finishedAt', finishedAt)
          ..add('id', id)
          ..add('inputBytes', inputBytes)
          ..add('nodeId', nodeId)
          ..add('openedAt', openedAt)
          ..add('outputBytes', outputBytes)
          ..add('startedAt', startedAt)
          ..add('status', status))
        .toString();
  }
}

class TerminalSessionResponseBuilder
    implements
        Builder<TerminalSessionResponse, TerminalSessionResponseBuilder> {
  _$TerminalSessionResponse? _$v;

  String? _actorId;
  String? get actorId => _$this._actorId;
  set actorId(String? actorId) => _$this._actorId = actorId;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  String? _closeRequestedAt;
  String? get closeRequestedAt => _$this._closeRequestedAt;
  set closeRequestedAt(String? closeRequestedAt) =>
      _$this._closeRequestedAt = closeRequestedAt;

  int? _exitCode;
  int? get exitCode => _$this._exitCode;
  set exitCode(int? exitCode) => _$this._exitCode = exitCode;

  String? _exitReason;
  String? get exitReason => _$this._exitReason;
  set exitReason(String? exitReason) => _$this._exitReason = exitReason;

  String? _finishedAt;
  String? get finishedAt => _$this._finishedAt;
  set finishedAt(String? finishedAt) => _$this._finishedAt = finishedAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  int? _inputBytes;
  int? get inputBytes => _$this._inputBytes;
  set inputBytes(int? inputBytes) => _$this._inputBytes = inputBytes;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _openedAt;
  String? get openedAt => _$this._openedAt;
  set openedAt(String? openedAt) => _$this._openedAt = openedAt;

  int? _outputBytes;
  int? get outputBytes => _$this._outputBytes;
  set outputBytes(int? outputBytes) => _$this._outputBytes = outputBytes;

  String? _startedAt;
  String? get startedAt => _$this._startedAt;
  set startedAt(String? startedAt) => _$this._startedAt = startedAt;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  TerminalSessionResponseBuilder() {
    TerminalSessionResponse._defaults(this);
  }

  TerminalSessionResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _actorId = $v.actorId;
      _agentId = $v.agentId;
      _closeRequestedAt = $v.closeRequestedAt;
      _exitCode = $v.exitCode;
      _exitReason = $v.exitReason;
      _finishedAt = $v.finishedAt;
      _id = $v.id;
      _inputBytes = $v.inputBytes;
      _nodeId = $v.nodeId;
      _openedAt = $v.openedAt;
      _outputBytes = $v.outputBytes;
      _startedAt = $v.startedAt;
      _status = $v.status;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TerminalSessionResponse other) {
    _$v = other as _$TerminalSessionResponse;
  }

  @override
  void update(void Function(TerminalSessionResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TerminalSessionResponse build() => _build();

  _$TerminalSessionResponse _build() {
    final _$result =
        _$v ??
        _$TerminalSessionResponse._(
          actorId: BuiltValueNullFieldError.checkNotNull(
            actorId,
            r'TerminalSessionResponse',
            'actorId',
          ),
          agentId: BuiltValueNullFieldError.checkNotNull(
            agentId,
            r'TerminalSessionResponse',
            'agentId',
          ),
          closeRequestedAt: closeRequestedAt,
          exitCode: exitCode,
          exitReason: exitReason,
          finishedAt: finishedAt,
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'TerminalSessionResponse',
            'id',
          ),
          inputBytes: BuiltValueNullFieldError.checkNotNull(
            inputBytes,
            r'TerminalSessionResponse',
            'inputBytes',
          ),
          nodeId: BuiltValueNullFieldError.checkNotNull(
            nodeId,
            r'TerminalSessionResponse',
            'nodeId',
          ),
          openedAt: openedAt,
          outputBytes: BuiltValueNullFieldError.checkNotNull(
            outputBytes,
            r'TerminalSessionResponse',
            'outputBytes',
          ),
          startedAt: BuiltValueNullFieldError.checkNotNull(
            startedAt,
            r'TerminalSessionResponse',
            'startedAt',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'TerminalSessionResponse',
            'status',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
