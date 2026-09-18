// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'source_materialization.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$SourceMaterialization extends SourceMaterialization {
  @override
  final SourceMaterializationMode mode;
  @override
  final BuiltList<String>? paths;

  factory _$SourceMaterialization([
    void Function(SourceMaterializationBuilder)? updates,
  ]) => (SourceMaterializationBuilder()..update(updates))._build();

  _$SourceMaterialization._({required this.mode, this.paths}) : super._();
  @override
  SourceMaterialization rebuild(
    void Function(SourceMaterializationBuilder) updates,
  ) => (toBuilder()..update(updates)).build();

  @override
  SourceMaterializationBuilder toBuilder() =>
      SourceMaterializationBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is SourceMaterialization &&
        mode == other.mode &&
        paths == other.paths;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, mode.hashCode);
    _$hash = $jc(_$hash, paths.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(r'SourceMaterialization')
          ..add('mode', mode)
          ..add('paths', paths))
        .toString();
  }
}

class SourceMaterializationBuilder
    implements Builder<SourceMaterialization, SourceMaterializationBuilder> {
  _$SourceMaterialization? _$v;

  SourceMaterializationMode? _mode;
  SourceMaterializationMode? get mode => _$this._mode;
  set mode(SourceMaterializationMode? mode) => _$this._mode = mode;

  ListBuilder<String>? _paths;
  ListBuilder<String> get paths => _$this._paths ??= ListBuilder<String>();
  set paths(ListBuilder<String>? paths) => _$this._paths = paths;

  SourceMaterializationBuilder() {
    SourceMaterialization._defaults(this);
  }

  SourceMaterializationBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _mode = $v.mode;
      _paths = $v.paths?.toBuilder();
      _$v = null;
    }
    return this;
  }

  @override
  void replace(SourceMaterialization other) {
    _$v = other as _$SourceMaterialization;
  }

  @override
  void update(void Function(SourceMaterializationBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  SourceMaterialization build() => _build();

  _$SourceMaterialization _build() {
    _$SourceMaterialization _$result;
    try {
      _$result =
          _$v ??
          _$SourceMaterialization._(
            mode: BuiltValueNullFieldError.checkNotNull(
              mode,
              r'SourceMaterialization',
              'mode',
            ),
            paths: _paths?.build(),
          );
    } catch (_) {
      late String _$failedField;
      try {
        _$failedField = 'paths';
        _paths?.build();
      } catch (e) {
        throw BuiltValueNestedFieldError(
          r'SourceMaterialization',
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
