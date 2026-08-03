// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_install_command_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentInstallCommandResponse extends AgentInstallCommandResponse {
  @override
  final String agentId;
  @override
  final String enrollmentExpiresAt;
  @override
  final String enrollmentToken;
  @override
  final String installCommand;

  factory _$AgentInstallCommandResponse([
    void Function(AgentInstallCommandResponseBuilder)? updates,
  ]) => (AgentInstallCommandResponseBuilder()..update(updates))._build();

  _$AgentInstallCommandResponse._({
    required this.agentId,
    required this.enrollmentExpiresAt,
    required this.enrollmentToken,
    required this.installCommand,
  }) : super._();
  @override
  AgentInstallCommandResponse rebuild(
    void Function(AgentInstallCommandResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  AgentInstallCommandResponseBuilder toBuilder() =>
      AgentInstallCommandResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentInstallCommandResponse &&
        agentId == other.agentId &&
        enrollmentExpiresAt == other.enrollmentExpiresAt &&
        enrollmentToken == other.enrollmentToken &&
        installCommand == other.installCommand;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agentId.hashCode);
    _$hash = $jc(_$hash, enrollmentExpiresAt.hashCode);
    _$hash = $jc(_$hash, enrollmentToken.hashCode);
    _$hash = $jc(_$hash, installCommand.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentInstallCommandResponse')
          ..add('agentId', agentId)
          ..add('enrollmentExpiresAt', enrollmentExpiresAt)
          ..add('enrollmentToken', enrollmentToken)
          ..add('installCommand', installCommand))
        .toString();
  }
}

class AgentInstallCommandResponseBuilder
    implements
        Builder<
          AgentInstallCommandResponse,
          AgentInstallCommandResponseBuilder
        > {
  _$AgentInstallCommandResponse? _$v;

  String? _agentId;
  String? get agentId => _$this._agentId;
  set agentId(String? agentId) => _$this._agentId = agentId;

  String? _enrollmentExpiresAt;
  String? get enrollmentExpiresAt => _$this._enrollmentExpiresAt;
  set enrollmentExpiresAt(String? enrollmentExpiresAt) =>
      _$this._enrollmentExpiresAt = enrollmentExpiresAt;

  String? _enrollmentToken;
  String? get enrollmentToken => _$this._enrollmentToken;
  set enrollmentToken(String? enrollmentToken) =>
      _$this._enrollmentToken = enrollmentToken;

  String? _installCommand;
  String? get installCommand => _$this._installCommand;
  set installCommand(String? installCommand) =>
      _$this._installCommand = installCommand;

  AgentInstallCommandResponseBuilder() {
    AgentInstallCommandResponse._defaults(this);
  }

  AgentInstallCommandResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agentId = $v.agentId;
      _enrollmentExpiresAt = $v.enrollmentExpiresAt;
      _enrollmentToken = $v.enrollmentToken;
      _installCommand = $v.installCommand;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentInstallCommandResponse other) {
    _$v = other as _$AgentInstallCommandResponse;
  }

  @override
  void update(void Function(AgentInstallCommandResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentInstallCommandResponse build() => _build();

  _$AgentInstallCommandResponse _build() {
    final _$result =
        _$v ??
        _$AgentInstallCommandResponse._(
          agentId: BuiltValueNullFieldError.checkNotNull(
            agentId,
            r'AgentInstallCommandResponse',
            'agentId',
          ),
          enrollmentExpiresAt: BuiltValueNullFieldError.checkNotNull(
            enrollmentExpiresAt,
            r'AgentInstallCommandResponse',
            'enrollmentExpiresAt',
          ),
          enrollmentToken: BuiltValueNullFieldError.checkNotNull(
            enrollmentToken,
            r'AgentInstallCommandResponse',
            'enrollmentToken',
          ),
          installCommand: BuiltValueNullFieldError.checkNotNull(
            installCommand,
            r'AgentInstallCommandResponse',
            'installCommand',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
