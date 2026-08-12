// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'save_application_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SaveApplicationRequest extends SaveApplicationRequest {
  @override
  final String? description;
  @override
  final String environment;
  @override
  final String name;
  @override
  final String slug;
  @override
  final int? version;

  factory _$SaveApplicationRequest([
    void Function(SaveApplicationRequestBuilder)? updates,
  ]) => (SaveApplicationRequestBuilder()..update(updates))._build();

  _$SaveApplicationRequest._({
    this.description,
    required this.environment,
    required this.name,
    required this.slug,
    this.version,
  }) : super._();
  @override
  SaveApplicationRequest rebuild(
    void Function(SaveApplicationRequestBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  SaveApplicationRequestBuilder toBuilder() =>
      SaveApplicationRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SaveApplicationRequest &&
        description == other.description &&
        environment == other.environment &&
        name == other.name &&
        slug == other.slug &&
        version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, environment.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SaveApplicationRequest')
          ..add('description', description)
          ..add('environment', environment)
          ..add('name', name)
          ..add('slug', slug)
          ..add('version', version))
        .toString();
  }
}

class SaveApplicationRequestBuilder
    implements Builder<SaveApplicationRequest, SaveApplicationRequestBuilder> {
  _$SaveApplicationRequest? _$v;

  String? _description;
  String? get description => _$this._description;
  set description(String? description) => _$this._description = description;

  String? _environment;
  String? get environment => _$this._environment;
  set environment(String? environment) => _$this._environment = environment;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  String? _slug;
  String? get slug => _$this._slug;
  set slug(String? slug) => _$this._slug = slug;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  SaveApplicationRequestBuilder() {
    SaveApplicationRequest._defaults(this);
  }

  SaveApplicationRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _environment = $v.environment;
      _name = $v.name;
      _slug = $v.slug;
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SaveApplicationRequest other) {
    _$v = other as _$SaveApplicationRequest;
  }

  @override
  void update(void Function(SaveApplicationRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SaveApplicationRequest build() => _build();

  _$SaveApplicationRequest _build() {
    final _$result =
        _$v ??
        _$SaveApplicationRequest._(
          description: description,
          environment: BuiltValueNullFieldError.checkNotNull(
            environment,
            r'SaveApplicationRequest',
            'environment',
          ),
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'SaveApplicationRequest',
            'name',
          ),
          slug: BuiltValueNullFieldError.checkNotNull(
            slug,
            r'SaveApplicationRequest',
            'slug',
          ),
          version: version,
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
