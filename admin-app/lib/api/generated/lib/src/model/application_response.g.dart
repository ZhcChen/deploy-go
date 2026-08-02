// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'application_response.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ApplicationResponse extends ApplicationResponse {
  @override
  final String createdAt;
  @override
  final String description;
  @override
  final String id;
  @override
  final String name;
  @override
  final String slug;
  @override
  final String status;
  @override
  final String updatedAt;
  @override
  final int version;

  factory _$ApplicationResponse([
    void Function(ApplicationResponseBuilder)? updates,
  ]) => (ApplicationResponseBuilder()..update(updates))._build();

  _$ApplicationResponse._({
    required this.createdAt,
    required this.description,
    required this.id,
    required this.name,
    required this.slug,
    required this.status,
    required this.updatedAt,
    required this.version,
  }) : super._();
  @override
  ApplicationResponse rebuild(
    void Function(ApplicationResponseBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ApplicationResponseBuilder toBuilder() =>
      ApplicationResponseBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ApplicationResponse &&
        createdAt == other.createdAt &&
        description == other.description &&
        id == other.id &&
        name == other.name &&
        slug == other.slug &&
        status == other.status &&
        updatedAt == other.updatedAt &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, createdAt.hashCode);
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, updatedAt.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ApplicationResponse')
          ..add('createdAt', createdAt)
          ..add('description', description)
          ..add('id', id)
          ..add('name', name)
          ..add('slug', slug)
          ..add('status', status)
          ..add('updatedAt', updatedAt)
          ..add('version', version))
        .toString();
  }
}

class ApplicationResponseBuilder
    implements Builder<ApplicationResponse, ApplicationResponseBuilder> {
  _$ApplicationResponse? _$v;

  String? _createdAt;
  String? get createdAt => _$this._createdAt;
  set createdAt(String? createdAt) => _$this._createdAt = createdAt;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _id;
  String? get id => _$this._id;
  set id(String? id) => _$this._id = id;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  String? _status;
  String? get status => _$this._status;
  set status(String? status) => _$this._status = status;

  String? _updatedAt;
  String? get updatedAt => _$this._updatedAt;
  set updatedAt(String? updatedAt) => _$this._updatedAt = updatedAt;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  ApplicationResponseBuilder() {
    ApplicationResponse._defaults(this);
  }

  ApplicationResponseBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _createdAt = $v.createdAt;
      _description = $v.description;
      _id = $v.id;
      _name = $v.name;
      _slug = $v.slug;
      _status = $v.status;
      _updatedAt = $v.updatedAt;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ApplicationResponse other) {
    _$v = other as _$ApplicationResponse;
  }

  @override
  void update(void Function(ApplicationResponseBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ApplicationResponse build() => _build();

  _$ApplicationResponse _build() {
    final _$result =
        _$v ??
        _$ApplicationResponse._(
          createdAt: BuiltValueNullFieldError.checkNotNull(
            createdAt,
            r'ApplicationResponse',
            'createdAt',
          ),
          description: BuiltValueNullFieldError.checkNotNull(
            description,
            r'ApplicationResponse',
            'description',
          ),
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'ApplicationResponse',
            'id',
          ),
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'ApplicationResponse',
            'name',
          ),
          slug: BuiltValueNullFieldError.checkNotNull(
            slug,
            r'ApplicationResponse',
            'slug',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'ApplicationResponse',
            'status',
          ),
          updatedAt: BuiltValueNullFieldError.checkNotNull(
            updatedAt,
            r'ApplicationResponse',
            'updatedAt',
          ),
          version: BuiltValueNullFieldError.checkNotNull(
            version,
            r'ApplicationResponse',
            'version',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
