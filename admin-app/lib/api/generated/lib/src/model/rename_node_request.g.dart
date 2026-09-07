// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'rename_node_request.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

class _$RenameNodeRequest extends RenameNodeRequest {
  @override
  final String name;

  factory _$RenameNodeRequest([
    void Function(RenameNodeRequestBuilder)? updates,
  ]) => (RenameNodeRequestBuilder()..update(updates))._build();

  _$RenameNodeRequest._({required this.name}) : super._();
  @override
  RenameNodeRequest rebuild(void Function(RenameNodeRequestBuilder) updates) =>
      (toBuilder()..update(updates)).build();

  @override
  RenameNodeRequestBuilder toBuilder() =>
      RenameNodeRequestBuilder()..replace(this);

  @override
  bool operator ==(Object other) {
    if (identical(other, this)) return true;
    return other is RenameNodeRequest && name == other.name;
  }

  @override
  int get hashCode {
    var _$hash = 0;
    _$hash = $jc(_$hash, name.hashCode);
    _$hash = $jf(_$hash);
    return _$hash;
  }

  @override
  String toString() {
    return (newBuiltValueToStringHelper(
      r'RenameNodeRequest',
    )..add('name', name)).toString();
  }
}

class RenameNodeRequestBuilder
    implements Builder<RenameNodeRequest, RenameNodeRequestBuilder> {
  _$RenameNodeRequest? _$v;

  String? _name;
  String? get name => _$this._name;
  set name(String? name) => _$this._name = name;

  RenameNodeRequestBuilder() {
    RenameNodeRequest._defaults(this);
  }

  RenameNodeRequestBuilder get _$this {
    final $v = _$v;
    if ($v != null) {
      _name = $v.name;
      _$v = null;
    }
    return this;
  }

  @override
  void replace(RenameNodeRequest other) {
    _$v = other as _$RenameNodeRequest;
  }

  @override
  void update(void Function(RenameNodeRequestBuilder)? updates) {
    if (updates != null) updates(this);
  }

  @override
  RenameNodeRequest build() => _build();

  _$RenameNodeRequest _build() {
    final _$result =
        _$v ??
        _$RenameNodeRequest._(
          name: BuiltValueNullFieldError.checkNotNull(
            name,
            r'RenameNodeRequest',
            'name',
          ),
        );
    replace(_$result);
    return _$result;
  }
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
