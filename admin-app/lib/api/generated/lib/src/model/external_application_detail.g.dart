// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'external_application_detail.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ExternalApplicationDetail extends ExternalApplicationDetail {
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
  final BuiltList<ExternalDeploymentTarget> targets;

  factory _$ExternalApplicationDetail([
    void Function(ExternalApplicationDetailBuilder)? updates,
  ]) => (ExternalApplicationDetailBuilder()..update(updates))._build();

  _$ExternalApplicationDetail._({
    required this.description,
    required this.id,
    required this.name,
    required this.slug,
    required this.status,
    required this.targets,
  }) : super._();
  @override
  ExternalApplicationDetail rebuild(
    void Function(ExternalApplicationDetailBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ExternalApplicationDetailBuilder toBuilder() =>
      ExternalApplicationDetailBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ExternalApplicationDetail &&
        description == other.description &&
        id == other.id &&
        name == other.name &&
        slug == other.slug &&
        status == other.status &&
        targets == other.targets;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jc(_$hash, targets.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ExternalApplicationDetail')
          ..add('description', description)
          ..add('id', id)
          ..add('name', name)
          ..add('slug', slug)
          ..add('status', status)
          ..add('targets', targets))
        .toString();
  }
}

class ExternalApplicationDetailBuilder
    implements
        Builder<ExternalApplicationDetail, ExternalApplicationDetailBuilder> {
  _$ExternalApplicationDetail? _$v;

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

  ListBuilder<ExternalDeploymentTarget>? _targets;
  ListBuilder<ExternalDeploymentTarget> get targets =>
      _$this._targets ??= ListBuilder<ExternalDeploymentTarget>();
  set targets(ListBuilder<ExternalDeploymentTarget>? targets) =>
      _$this._targets = targets;

  ExternalApplicationDetailBuilder() {
    ExternalApplicationDetail._defaults(this);
  }

  ExternalApplicationDetailBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _id = $v.id;
      _name = $v.name;
      _slug = $v.slug;
      _status = $v.status;
      _targets = $v.targets.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ExternalApplicationDetail other) {
    _$v = other as _$ExternalApplicationDetail;
  }

  @override
  void update(void Function(ExternalApplicationDetailBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ExternalApplicationDetail build() => _build();

  _$ExternalApplicationDetail _build() {
    _$ExternalApplicationDetail _$result;
    try {
      _$result =
          _$v ??
          _$ExternalApplicationDetail._(
            description: BuiltValueNullFieldError.checkNotNull(
              description,
              r'ExternalApplicationDetail',
              'description',
            ),
            id: BuiltValueNullFieldError.checkNotNull(
              id,
              r'ExternalApplicationDetail',
              'id',
            ),
            name: BuiltValueNullFieldError.checkNotNull(
              name,
              r'ExternalApplicationDetail',
              'name',
            ),
            slug: BuiltValueNullFieldError.checkNotNull(
              slug,
              r'ExternalApplicationDetail',
              'slug',
            ),
            status: BuiltValueNullFieldError.checkNotNull(
              status,
              r'ExternalApplicationDetail',
              'status',
            ),
            targets: targets.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'targets';
        targets.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'ExternalApplicationDetail',
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
