// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'external_application_summary.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ExternalApplicationSummary extends ExternalApplicationSummary {
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

  factory _$ExternalApplicationSummary([
    void Function(ExternalApplicationSummaryBuilder)? updates,
  ]) => (ExternalApplicationSummaryBuilder()..update(updates))._build();

  _$ExternalApplicationSummary._({
    required this.description,
    required this.id,
    required this.name,
    required this.slug,
    required this.status,
  }) : super._();
  @override
  ExternalApplicationSummary rebuild(
    void Function(ExternalApplicationSummaryBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  ExternalApplicationSummaryBuilder toBuilder() =>
      ExternalApplicationSummaryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ExternalApplicationSummary &&
        description == other.description &&
        id == other.id &&
        name == other.name &&
        slug == other.slug &&
        status == other.status;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, description.hashCode);
    _$hash = $jc(_$hash, id.hashCode);
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jc(_$hash, slug.hashCode);
    _$hash = $jc(_$hash, status.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'ExternalApplicationSummary')
          ..add('description', description)
          ..add('id', id)
          ..add('name', name)
          ..add('slug', slug)
          ..add('status', status))
        .toString();
  }
}

class ExternalApplicationSummaryBuilder
    implements
        Builder<ExternalApplicationSummary, ExternalApplicationSummaryBuilder> {
  _$ExternalApplicationSummary? _$v;

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

  ExternalApplicationSummaryBuilder() {
    ExternalApplicationSummary._defaults(this);
  }

  ExternalApplicationSummaryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _description = $v.description;
      _id = $v.id;
      _name = $v.name;
      _slug = $v.slug;
      _status = $v.status;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ExternalApplicationSummary other) {
    _$v = other as _$ExternalApplicationSummary;
  }

  @override
  void update(void Function(ExternalApplicationSummaryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ExternalApplicationSummary build() => _build();

  _$ExternalApplicationSummary _build() {
    final _$result =
        _$v ??
        _$ExternalApplicationSummary._(
          description: BuiltValueNullFieldError.checkNotNull(
            description,
            r'ExternalApplicationSummary',
            'description',
          ),
          id: BuiltValueNullFieldError.checkNotNull(
            id,
            r'ExternalApplicationSummary',
            'id',
          ),
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'ExternalApplicationSummary',
            'name',
          ),
          slug: BuiltValueNullFieldError.checkNotNull(
            slug,
            r'ExternalApplicationSummary',
            'slug',
          ),
          status: BuiltValueNullFieldError.checkNotNull(
            status,
            r'ExternalApplicationSummary',
            'status',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
