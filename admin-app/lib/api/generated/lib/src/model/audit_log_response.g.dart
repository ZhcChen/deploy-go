// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'audit_log_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$AuditLogResponse extends AuditLogResponse {
  @override
  final String action;
  @override
  final String? actorId;
  @override
  final String createdAt;
  @override
  final String id;
  @override
  final String requestId;
  @override
  final String resourceId;
  @override
  final String resourceType;
  @override
  final JsonObject? summary;

  factory _$AuditLogResponse([
    void Function(AuditLogResponseBuilder)? updates,
  ]) => (AuditLogResponseBuilder()..update(updates))._build();

  _$AuditLogResponse._({
    required this.action,
    this.actorId,
    required this.createdAt,
    required this.id,
    required this.requestId,
    required this.resourceId,
    required this.resourceType,
    this.summary,
  }) : super._();
  @override
  AuditLogResponse rebuild(void Function(AuditLogResponseBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  AuditLogResponseBuilder toBuilder() =>
      AuditLogResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is AuditLogResponse &&
        action == other.action &&
        actorId == other.actorId &&
        createdAt == other.createdAt &&
        id == other.id &&
        requestId == other.requestId &&
        resourceId == other.resourceId &&
        resourceType == other.resourceType &&
        summary == other.summary;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, action.hashCode);
    _$hash = $jc(_$hash, actorId.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, requestId.hashCode);
    _$hash = $jc(_$hash, resourceId.hashCode);
    _$hash = $jc(_$hash, resourceType.hashCode);
    _$hash = $jc(_$hash, summary.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'AuditLogResponse')
          ..add('action', action)
          ..add('actorId', actorId)
          ..add('createdAt', createdAt)
          ..add('id', id)
          ..add('requestId', requestId)
          ..add('resourceId', resourceId)
          ..add('resourceType', resourceType)
          ..add('summary', summary))
        .toString();
  }
}

class AuditLogResponseBuilder
    implements Builder<AuditLogResponse, AuditLogResponseBuilder> {
  _$AuditLogResponse? _$v;

  String? _action;
  String? get action => _$this._action;
  set action(String? action) => _$this._action = action;

  String? _actorId;
  String? get actorId => _$this._actorId;
  set actorId(String? actorId) => _$this._actorId = actorId;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _requestId;
  String? get requestId => _$this._requestId;
  set requestId(String? requestId) => _$this._requestId = requestId;

  String? _resourceId;
  String? get resourceId => _$this._resourceId;
  set resourceId(String? resourceId) => _$this._resourceId = resourceId;

  String? _resourceType;
  String? get resourceType => _$this._resourceType;
  set resourceType(String? resourceType) => _$this._resourceType = resourceType;

  JsonObject? _summary;
  JsonObject? get summary => _$this._summary;
  set summary(JsonObject? summary) => _$this._summary = summary;

  AuditLogResponseBuilder() {
    AuditLogResponse._defaults(this);
  }

  AuditLogResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _action = $v.action;
      _actorId = $v.actorId;
      _createdAt = $v.createdAt;
      _id = $v.id;
      _requestId = $v.requestId;
      _resourceId = $v.resourceId;
      _resourceType = $v.resourceType;
      _summary = $v.summary;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(AuditLogResponse other) {
    _$v = other as _$AuditLogResponse;
  }

  @override
  void update(void Function(AuditLogResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  AuditLogResponse build() => _build();

  _$AuditLogResponse _build() {
    final _$result =
        _$v ??
        _$AuditLogResponse._(
          action: BuiltValueNullFieldError.checkNotNull(
            action,
            r'AuditLogResponse',
            'action',
          ),
          actorId: actorId,
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'AuditLogResponse',
            'createdAt',
          ),
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'AuditLogResponse',
            'id',
          ),
          requestId: BuiltValueNullFieldError.checkNotNull(
            requestId,
            r'AuditLogResponse',
            'requestId',
          ),
          resourceId: BuiltValueNullFieldError.checkNotNull(
            resourceId,
            r'AuditLogResponse',
            'resourceId',
          ),
          resourceType: BuiltValueNullFieldError.checkNotNull(
            resourceType,
            r'AuditLogResponse',
            'resourceType',
          ),
          summary: summary,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
