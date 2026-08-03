// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_enrollment_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentEnrollmentResponse extends AgentEnrollmentResponse {
  @override
  final AgentResponse agent;
  @override
  final String enrollmentExpiresAt;
  @override
  final String enrollmentToken;
  @override
  final String installCommand;

  factory _$AgentEnrollmentResponse([
    void Function(AgentEnrollmentResponseBuilder)? updates,
  ]) => (AgentEnrollmentResponseBuilder()..update(updates))._build();

  _$AgentEnrollmentResponse._({
    required this.agent,
    required this.enrollmentExpiresAt,
    required this.enrollmentToken,
    required this.installCommand,
  }) : super._();
  @override
  AgentEnrollmentResponse rebuild(
    void Function(AgentEnrollmentResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  AgentEnrollmentResponseBuilder toBuilder() =>
      AgentEnrollmentResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentEnrollmentResponse &&
        agent == other.agent &&
        enrollmentExpiresAt == other.enrollmentExpiresAt &&
        enrollmentToken == other.enrollmentToken &&
        installCommand == other.installCommand;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, agent.hashCode);
    _$hash = $jc(_$hash, enrollmentExpiresAt.hashCode);
    _$hash = $jc(_$hash, enrollmentToken.hashCode);
    _$hash = $jc(_$hash, installCommand.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentEnrollmentResponse')
          ..add('agent', agent)
          ..add('enrollmentExpiresAt', enrollmentExpiresAt)
          ..add('enrollmentToken', enrollmentToken)
          ..add('installCommand', installCommand))
        .toString();
  }
}

class AgentEnrollmentResponseBuilder
    implements
        Builder<AgentEnrollmentResponse, AgentEnrollmentResponseBuilder> {
  _$AgentEnrollmentResponse? _$v;

  AgentResponseBuilder? _agent;
  AgentResponseBuilder get agent => _$this._agent ??= AgentResponseBuilder();
  set agent(AgentResponseBuilder? agent) => _$this._agent = agent;

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

  AgentEnrollmentResponseBuilder() {
    AgentEnrollmentResponse._defaults(this);
  }

  AgentEnrollmentResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _agent = $v.agent.toBuilder();
      _enrollmentExpiresAt = $v.enrollmentExpiresAt;
      _enrollmentToken = $v.enrollmentToken;
      _installCommand = $v.installCommand;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentEnrollmentResponse other) {
    _$v = other as _$AgentEnrollmentResponse;
  }

  @override
  void update(void Function(AgentEnrollmentResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentEnrollmentResponse build() => _build();

  _$AgentEnrollmentResponse _build() {
    _$AgentEnrollmentResponse _$result;
    try {
      _$result =
          _$v ??
          _$AgentEnrollmentResponse._(
            agent: agent.build(),
            enrollmentExpiresAt: BuiltValueNullFieldError.checkNotNull(
              enrollmentExpiresAt,
              r'AgentEnrollmentResponse',
              'enrollmentExpiresAt',
            ),
            enrollmentToken: BuiltValueNullFieldError.checkNotNull(
              enrollmentToken,
              r'AgentEnrollmentResponse',
              'enrollmentToken',
            ),
            installCommand: BuiltValueNullFieldError.checkNotNull(
              installCommand,
              r'AgentEnrollmentResponse',
              'installCommand',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'agent';
        agent.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'AgentEnrollmentResponse',
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
