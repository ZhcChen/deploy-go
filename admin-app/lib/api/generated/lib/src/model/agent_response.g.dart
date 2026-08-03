// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'agent_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AgentResponse extends AgentResponse {
  @override
  final String id;
  @override
  final String? lastSeenAt;
  @override
  final String name;
  @override
  final String nodeId;
  @override
  final String? registeredAt;
  @override
  final String status;

  factory _$AgentResponse([void Function(AgentResponseBuilder)? updates]) =>
      (AgentResponseBuilder()..update(updates))._build();

  _$AgentResponse._({
    required this.id,
    this.lastSeenAt,
    required this.name,
    required this.nodeId,
    this.registeredAt,
    required this.status,
  }) : super._();
  @override
  AgentResponse rebuild(void Function(AgentResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AgentResponseBuilder toBuilder() => AgentResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AgentResponse &&
        id == other.id &&
        lastSeenAt == other.lastSeenAt &&
        name == other.name &&
        nodeId == other.nodeId &&
        registeredAt == other.registeredAt &&
        status == other.status;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, lastSeenAt.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, nodeId.hashCode);
    _$hash = $jc(_$hash, registeredAt.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AgentResponse')
          ..add('id', id)
          ..add('lastSeenAt', lastSeenAt)
          ..add('name', name)
          ..add('nodeId', nodeId)
          ..add('registeredAt', registeredAt)
          ..add('status', status))
        .toString();
  }
}

class AgentResponseBuilder
    implements Builder<AgentResponse, AgentResponseBuilder> {
  _$AgentResponse? _$v;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _lastSeenAt;
  String? get lastSeenAt => _$this._lastSeenAt;
  set lastSeenAt(String? lastSeenAt) => _$this._lastSeenAt = lastSeenAt;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _nodeId;
  String? get nodeId => _$this._nodeId;
  set nodeId(String? nodeId) => _$this._nodeId = nodeId;

  String? _registeredAt;
  String? get registeredAt => _$this._registeredAt;
  set registeredAt(String? registeredAt) => _$this._registeredAt = registeredAt;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  AgentResponseBuilder() {
    AgentResponse._defaults(this);
  }

  AgentResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _id = $v.id;
      _lastSeenAt = $v.lastSeenAt;
      _name = $v.name;
      _nodeId = $v.nodeId;
      _registeredAt = $v.registeredAt;
      _status = $v.status;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AgentResponse other) {
    _$v = other as _$AgentResponse;
  }

  @override
  void update(void Function(AgentResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AgentResponse build() => _build();

  _$AgentResponse _build() {
    final _$result =
        _$v ??
        _$AgentResponse._(
          id: BuiltValueNullFieldError.checkNotNull(id, r'AgentResponse', 'id'),
          lastSeenAt: lastSeenAt,
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'AgentResponse',
            'name',
          ),
          nodeId: BuiltValueNullFieldError.checkNotNull(
            nodeId,
            r'AgentResponse',
            'nodeId',
          ),
          registeredAt: registeredAt,
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'AgentResponse',
            'status',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
