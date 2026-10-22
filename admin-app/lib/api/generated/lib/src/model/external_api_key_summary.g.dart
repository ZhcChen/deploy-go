// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'external_api_key_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ExternalApiKeySummary extends ExternalApiKeySummary {
  @override
  final BuiltList<String> applicationIds;
  @override
  final String createdAt;
  @override
  final String? expiresAt;
  @override
  final String id;
  @override
  final String? lastUsedAt;
  @override
  final String name;
  @override
  final String status;
  @override
  final String updatedAt;
  @override
  final int version;

  factory _$ExternalApiKeySummary([
    void Function(ExternalApiKeySummaryBuilder)? updates,
  ]) => (ExternalApiKeySummaryBuilder()..update(updates))._build();

  _$ExternalApiKeySummary._({
    required this.applicationIds,
    required this.createdAt,
    this.expiresAt,
    required this.id,
    this.lastUsedAt,
    required this.name,
    required this.status,
    required this.updatedAt,
    required this.version,
  }) : super._();
  @override
  ExternalApiKeySummary rebuild(
    void Function(ExternalApiKeySummaryBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ExternalApiKeySummaryBuilder toBuilder() =>
      ExternalApiKeySummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ExternalApiKeySummary &&
        applicationIds == other.applicationIds &&
        createdAt == other.createdAt &&
        expiresAt == other.expiresAt &&
        id == other.id &&
        lastUsedAt == other.lastUsedAt &&
        name == other.name &&
        status == other.status &&
        updatedAt == other.updatedAt &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, applicationIds.hashCode);
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, expiresAt.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, lastUsedAt.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ExternalApiKeySummary')
          ..add('applicationIds', applicationIds)
          ..add('createdAt', createdAt)
          ..add('expiresAt', expiresAt)
          ..add('id', id)
          ..add('lastUsedAt', lastUsedAt)
          ..add('name', name)
          ..add('status', status)
          ..add('updatedAt', updatedAt)
          ..add('version', version))
        .toString();
  }
}

class ExternalApiKeySummaryBuilder
    implements Builder<ExternalApiKeySummary, ExternalApiKeySummaryBuilder> {
  _$ExternalApiKeySummary? _$v;

  ListBuilder<String>? _applicationIds;
  ListBuilder<String> get applicationIds =>
      _$this._applicationIds ??= ListBuilder<String>();
  set applicationIds(ListBuilder<String>? applicationIds) =>
      _$this._applicationIds = applicationIds;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _expiresAt;
  String? get expiresAt => _$this._expiresAt;
  set expiresAt(String? expiresAt) => _$this._expiresAt = expiresAt;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _lastUsedAt;
  String? get lastUsedAt => _$this._lastUsedAt;
  set lastUsedAt(String? lastUsedAt) => _$this._lastUsedAt = lastUsedAt;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  ExternalApiKeySummaryBuilder() {
    ExternalApiKeySummary._defaults(this);
  }

  ExternalApiKeySummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _applicationIds = $v.applicationIds.toBuilder();
      _createdAt = $v.createdAt;
      _expiresAt = $v.expiresAt;
      _id = $v.id;
      _lastUsedAt = $v.lastUsedAt;
      _name = $v.name;
      _status = $v.status;
      _updatedAt = $v.updatedAt;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ExternalApiKeySummary other) {
    _$v = other as _$ExternalApiKeySummary;
  }

  @override
  void update(void Function(ExternalApiKeySummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ExternalApiKeySummary build() => _build();

  _$ExternalApiKeySummary _build() {
    _$ExternalApiKeySummary _$result;
    try {
      _$result =
          _$v ??
          _$ExternalApiKeySummary._(
            applicationIds: applicationIds.build(),
            createdAt: BuiltValueNullFieldError.checkNotNull(
              createdAt,
              r'ExternalApiKeySummary',
              'createdAt',
            ),
            expiresAt: expiresAt,
            id: BuiltValueNullFieldError.checkNotNull(
              id,
              r'ExternalApiKeySummary',
              'id',
            ),
            lastUsedAt: lastUsedAt,
            name: BuiltValueNullFieldError.checkNotNull(
              name,
              r'ExternalApiKeySummary',
              'name',
            ),
            status: BuiltValueNullFieldError.checkNotNull(
              status,
              r'ExternalApiKeySummary',
              'status',
            ),
            updatedAt: BuiltValueNullFieldError.checkNotNull(
              updatedAt,
              r'ExternalApiKeySummary',
              'updatedAt',
            ),
            version: BuiltValueNullFieldError.checkNotNull(
              version,
              r'ExternalApiKeySummary',
              'version',
            ),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'applicationIds';
        applicationIds.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ExternalApiKeySummary',
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
