// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'config_diff_query.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$ConfigDiffQuery extends ConfigDiffQuery {
  @override
  final int? version;

  factory _$ConfigDiffQuery([void Function(ConfigDiffQueryBuilder)? updates]) =>
      (ConfigDiffQueryBuilder()..update(updates))._build();

  _$ConfigDiffQuery._({this.version}) : super._();
  @override
  ConfigDiffQuery rebuild(void Function(ConfigDiffQueryBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  ConfigDiffQueryBuilder toBuilder() => ConfigDiffQueryBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is ConfigDiffQuery && version == other.version;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, version.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'ConfigDiffQuery',
    )..add('version', version)).toString();
  }
}

class ConfigDiffQueryBuilder
    implements Builder<ConfigDiffQuery, ConfigDiffQueryBuilder> {
  _$ConfigDiffQuery? _$v;

  int? _version;
  int? get version => _$this._version;
  set version(int? version) => _$this._version = version;

  ConfigDiffQueryBuilder() {
    ConfigDiffQuery._defaults(this);
  }

  ConfigDiffQueryBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _version = $v.version;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(ConfigDiffQuery other) {
    _$v = other as _$ConfigDiffQuery;
  }

  @override
  void update(void Function(ConfigDiffQueryBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  ConfigDiffQuery build() => _build();

  _$ConfigDiffQuery _build() {
    final _$result = _$v ?? _$ConfigDiffQuery._(version: version);
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
