// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'terminal_capability_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$TerminalCapabilityResponse extends TerminalCapabilityResponse {
  @override
  final String? agentId;
  @override
  final bool agentOnline;
  @override
  final bool available;
  @override
  final bool identityValid;
  @override
  final String nodeId;
  @override
  final bool privilegedExecution;
  @override
  final int? protocolVersion;
  @override
  final bool ptyTerminal;
  @override
  final String? unavailableCode;

  factory _$TerminalCapabilityResponse([
    void Function(TerminalCapabilityResponseBuilder)? updates,
  ]) => (TerminalCapabilityResponseBuilder()..update(updates))._build();

  _$TerminalCapabilityResponse._({
    this.agentId,
    required this.agentOnline,
    required this.available,
    required this.identityValid,
    required this.nodeId,
    required this.privilegedExecution,
    this.protocolVersion,
    required this.ptyTerminal,
    this.unavailableCode,
  }) : super._();
  @override
  TerminalCapabilityResponse rebuild(
    void Function(TerminalCapabilityResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  TerminalCapabilityResponseBuilder toBuilder() =>
      TerminalCapabilityResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is TerminalCapabilityResponse &&
        agentId == other.agentId &&
        agentOnline == other.agentOnline &&
        available == other.available &&
        identityValid == other.identityValid &&
        nodeId == other.nodeId &&
        privilegedExecution == other.privilegedExecution &&
        protocolVersion == other.protocolVersion &&
        ptyTerminal == other.ptyTerminal &&
        unavailableCode == other.unavailableCode;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, agentOnline.hashCode);
    _$hash = $jc(_$hash, available.hashCode);
    _$hash = $jc(_$hash, identityValid.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, privilegedExecution.hashCode);
    _$hash = $jc(_$hash, protocolVersion.hashCode);
    _$hash = $jc(_$hash, ptyTerminal.hashCode);
    _$hash = $jc(_$hash, unavailableCode.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'TerminalCapabilityResponse')
          ..add('agentId', agentId)
          ..add('agentOnline', agentOnline)
          ..add('available', available)
          ..add('identityValid', identityValid)
          ..add('nodeId', nodeId)
          ..add('privilegedExecution', privilegedExecution)
          ..add('protocolVersion', protocolVersion)
          ..add('ptyTerminal', ptyTerminal)
          ..add('unavailableCode', unavailableCode))
        .toString();
  }
}

class TerminalCapabilityResponseBuilder
    implements
        Builder<TerminalCapabilityResponse, TerminalCapabilityResponseBuilder> {
  _$TerminalCapabilityResponse? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  bool? _agentOnline;
  bool? get agentOnline => _$this._agentOnline;
  set agentOnline(bool? agentOnline) => _$this._agentOnline = agentOnline;

  bool? _available;
  bool? get available => _$this._available;
  set available(bool? available) => _$this._available = available;

  bool? _identityValid;
  bool? get identityValid => _$this._identityValid;
  set identityValid(bool? identityValid) =>
      _$this._identityValid = identityValid;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  bool? _privilegedExecution;
  bool? get privilegedExecution => _$this._privilegedExecution;
  set privilegedExecution(bool? privilegedExecution) =>
      _$this._privilegedExecution = privilegedExecution;

  int? _protocolVersion;
  int? get protocolVersion => _$this._protocolVersion;
  set protocolVersion(int? protocolVersion) =>
      _$this._protocolVersion = protocolVersion;

  bool? _ptyTerminal;
  bool? get ptyTerminal => _$this._ptyTerminal;
  set ptyTerminal(bool? ptyTerminal) => _$this._ptyTerminal = ptyTerminal;

  String? _unavailableCode;
  String? get unavailableCode => _$this._unavailableCode;
  set unavailableCode(String? unavailableCode) =>
      _$this._unavailableCode = unavailableCode;

  TerminalCapabilityResponseBuilder() {
    TerminalCapabilityResponse._defaults(this);
  }

  TerminalCapabilityResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _agentOnline = $v.agentOnline;
      _available = $v.available;
      _identityValid = $v.identityValid;
      _nodeId = $v.nodeId;
      _privilegedExecution = $v.privilegedExecution;
      _protocolVersion = $v.protocolVersion;
      _ptyTerminal = $v.ptyTerminal;
      _unavailableCode = $v.unavailableCode;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(TerminalCapabilityResponse other) {
    _$v = other as _$TerminalCapabilityResponse;
  }

  @override
  void update(void Function(TerminalCapabilityResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  TerminalCapabilityResponse build() => _build();

  _$TerminalCapabilityResponse _build() {
    final _$result =
        _$v ??
        _$TerminalCapabilityResponse._(
          agentId: agentId,
          agentOnline: BuiltValueNullFieldError.checkNotNull(
            agentOnline,
            r'TerminalCapabilityResponse',
            'agentOnline',
          ),
          available: BuiltValueNullFieldError.checkNotNull(
            available,
            r'TerminalCapabilityResponse',
            'available',
          ),
          identityValid: BuiltValueNullFieldError.checkNotNull(
            identityValid,
            r'TerminalCapabilityResponse',
            'identityValid',
          ),
          nodeId: BuiltValueNullFieldError.checkNotNull(
            nodeId,
            r'TerminalCapabilityResponse',
            'nodeId',
          ),
          privilegedExecution: BuiltValueNullFieldError.checkNotNull(
            privilegedExecution,
            r'TerminalCapabilityResponse',
            'privilegedExecution',
          ),
          protocolVersion: protocolVersion,
          ptyTerminal: BuiltValueNullFieldError.checkNotNull(
            ptyTerminal,
            r'TerminalCapabilityResponse',
            'ptyTerminal',
          ),
          unavailableCode: unavailableCode,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
