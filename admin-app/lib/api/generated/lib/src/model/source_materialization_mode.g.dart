// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'source_materialization_mode.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const SourceMaterializationMode _$full = const SourceMaterializationMode._(
  'full',
);
const SourceMaterializationMode _$sparse = const SourceMaterializationMode._(
  'sparse',
);

SourceMaterializationMode _$valueOf(String name) {
  switch (name) {
    case 'full':
      return _$full;
    case 'sparse':
      return _$sparse;
    default:
      throw ArgumentError(name);
  }
}

final BuiltSet<SourceMaterializationMode> _$values =
    BuiltSet<SourceMaterializationMode>(const <SourceMaterializationMode>[
      _$full,
      _$sparse,
    ]);

class _$SourceMaterializationModeMeta {
  const _$SourceMaterializationModeMeta();
  SourceMaterializationMode get full => _$full;
  SourceMaterializationMode get sparse => _$sparse;
  SourceMaterializationMode valueOf(String name) => _$valueOf(name);
  BuiltSet<SourceMaterializationMode> get values => _$values;
}

mixin _$SourceMaterializationModeMixin {
  // ignore: non_constant_identifier_names
  _$SourceMaterializationModeMeta get SourceMaterializationMode =>
      const _$SourceMaterializationModeMeta();
}

Serializer<SourceMaterializationMode> _$sourceMaterializationModeSerializer =
    _$SourceMaterializationModeSerializer();

class _$SourceMaterializationModeSerializer
    implements PrimitiveSerializer<SourceMaterializationMode> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'full': 'full',
    'sparse': 'sparse',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'full': 'full',
    'sparse': 'sparse',
  };

  @override
  final Iterable<Type> types = const <Type>[SourceMaterializationMode];
  @override
  final String wireName = 'SourceMaterializationMode';

  @override
  Object serialize(
    Serializers serializers,
    SourceMaterializationMode object, {
    FullType specifiedType = FullType.unspecified,
  }) => _toWire[object.name] ?? object.name;

  @override
  SourceMaterializationMode deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) => SourceMaterializationMode.valueOf(
    _fromWire[serialized] ?? (serialized is String ? serialized : ''),
  );
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
