// GENERATED CODE - DO NOT MODIFY BY HAND

part of 'image_template.dart';

// **************************************************************************
// BuiltValueGenerator
// **************************************************************************

const ImageTemplate _$redis = const ImageTemplate._('redis');
const ImageTemplate _$postgres = const ImageTemplate._('postgres');

ImageTemplate _$valueOf(String name) {
  switch (name) {
    case 'redis':
      return _$redis;
    case 'postgres':
      return _$postgres;
    default:
      throw ArgumentError(name);
  }
}

final BuiltSet<ImageTemplate> _$values = BuiltSet<ImageTemplate>(
  const <ImageTemplate>[_$redis, _$postgres],
);

class _$ImageTemplateMeta {
  const _$ImageTemplateMeta();
  ImageTemplate get redis => _$redis;
  ImageTemplate get postgres => _$postgres;
  ImageTemplate valueOf(String name) => _$valueOf(name);
  BuiltSet<ImageTemplate> get values => _$values;
}

mixin _$ImageTemplateMixin {
  // ignore: non_constant_identifier_names
  _$ImageTemplateMeta get ImageTemplate => const _$ImageTemplateMeta();
}

Serializer<ImageTemplate> _$imageTemplateSerializer =
    _$ImageTemplateSerializer();

class _$ImageTemplateSerializer implements PrimitiveSerializer<ImageTemplate> {
  static const Map<String, Object> _toWire = const <String, Object>{
    'redis': 'redis',
    'postgres': 'postgres',
  };
  static const Map<Object, String> _fromWire = const <Object, String>{
    'redis': 'redis',
    'postgres': 'postgres',
  };

  @override
  final Iterable<Type> types = const <Type>[ImageTemplate];
  @override
  final String wireName = 'ImageTemplate';

  @override
  Object serialize(
    Serializers serializers,
    ImageTemplate object, {
    FullType specifiedType = FullType.unspecified,
  }) => _toWire[object.name] ?? object.name;

  @override
  ImageTemplate deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) => ImageTemplate.valueOf(
    _fromWire[serialized] ?? (serialized is String ? serialized : ''),
  );
}

// ignore_for_file: deprecated_member_use_from_same_package,type=lint
